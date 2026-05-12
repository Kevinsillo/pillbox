//! Operaciones de store para la entidad [`Pill`].

use anyhow::{Context, Result};
use rusqlite::{params, Connection, TransactionBehavior};
use serde::Serialize;
use uuid::Uuid;

use crate::db::store::id_resolver::resolve_id;
use crate::domain::pill::{NewPill, Pill, PillPatch};
use crate::error::PillboxError;

// ─── Tipos de resultado ───────────────────────────────────────────────────────

/// Resultado de guardar una pill nueva.
#[derive(Debug, Serialize)]
pub struct PillStoreResult {
    pub id: String,
    pub action: &'static str, // "created"
    pub title: String,
    pub compound: String,
    pub content: String,
}

/// Resultado de descartar una pill (soft delete).
#[derive(Debug, Serialize)]
pub struct PillDiscardResult {
    pub id: String,
    pub deleted_at: String,
}

// ─── Store ────────────────────────────────────────────────────────────────────

/// Guarda una pill nueva en la prescription activa.
///
/// La prescription debe existir y estar abierta (`ended_at IS NULL`).
/// Falla con `prescription_required` si no se cumple.
pub fn take(conn: &mut Connection, input: &NewPill) -> Result<PillStoreResult> {
    // Resolver el prescription_id (acepta UUID completo o prefijo ≥8 chars) y
    // validar que existe y está abierta antes de insertar la pill.
    let resolved_rx_id = resolve_id(conn, "prescriptions", &input.prescription_id)?
        .ok_or_else(|| PillboxError::PrescriptionRequired {
            prescription_id: input.prescription_id.clone(),
        })?;

    let rx_open: bool = conn
        .query_row(
            "SELECT EXISTS(
             SELECT 1 FROM prescriptions
             WHERE id = ?1 AND ended_at IS NULL AND deleted_at IS NULL
         )",
            params![resolved_rx_id],
            |row| row.get(0),
        )
        .context("failed to check prescription status")?;

    if !rx_open {
        return Err(PillboxError::PrescriptionRequired {
            prescription_id: input.prescription_id.clone(),
        }
        .into());
    }

    let id = Uuid::now_v7().to_string();
    let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;

    tx.execute(
        "INSERT INTO pills
             (id, compound, title, content, prescription_id, author_name, author_email)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![
            id,
            input.compound.as_str(),
            input.title,
            input.content,
            resolved_rx_id,
            input.author_name,
            input.author_email,
        ],
    )
    .context("failed to insert pill")?;

    tx.commit()?;
    Ok(PillStoreResult {
        id,
        action: "created",
        title: input.title.clone(),
        compound: input.compound.as_str().to_string(),
        content: input.content.clone(),
    })
}

/// Lee una pill completa por UUID completo o prefijo ≥8 chars (no descartada).
pub fn read(conn: &Connection, id: &str) -> Result<Option<Pill>> {
    let Some(resolved_id) = resolve_id(conn, "pills", id)? else {
        return Ok(None);
    };
    match conn.query_row(
        "SELECT id, compound, title, content, prescription_id,
                author_name, author_email, created_at, updated_at, deleted_at
         FROM pills WHERE id = ?1 AND deleted_at IS NULL",
        params![resolved_id],
        row_to_pill,
    ) {
        Ok(p) => Ok(Some(p)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(e).context("failed to read pill"),
    }
}

/// Lee una pill completa por UUID completo o prefijo ≥8 chars, incluyendo descartadas.
pub fn read_any(conn: &Connection, id: &str) -> Result<Option<Pill>> {
    let Some(resolved_id) = resolve_id(conn, "pills", id)? else {
        return Ok(None);
    };
    match conn.query_row(
        "SELECT id, compound, title, content, prescription_id,
                author_name, author_email, created_at, updated_at, deleted_at
         FROM pills WHERE id = ?1",
        params![resolved_id],
        row_to_pill,
    ) {
        Ok(p) => Ok(Some(p)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(e).context("failed to read pill"),
    }
}

/// Actualiza campos de una pill existente (patch parcial).
///
/// Acepta UUID completo o prefijo ≥8 chars. Solo se modifican los campos
/// no-None. Devuelve `None` si no existe o fue descartada.
pub fn revise(conn: &mut Connection, id: &str, patch: &PillPatch) -> Result<Option<Pill>> {
    let Some(resolved_id) = resolve_id(conn, "pills", id)? else {
        return Ok(None);
    };

    let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;

    let affected = tx
        .execute(
            "UPDATE pills
         SET title    = COALESCE(?1, title),
             content  = COALESCE(?2, content),
             compound = COALESCE(?3, compound),
             updated_at = datetime('now')
         WHERE id = ?4 AND deleted_at IS NULL",
            params![
                patch.title.as_deref(),
                patch.content.as_deref(),
                patch.compound.as_ref().map(|c| c.as_str()),
                resolved_id,
            ],
        )
        .context("failed to update pill")?;

    if affected == 0 {
        return Ok(None);
    }

    let pill = tx
        .query_row(
            "SELECT id, compound, title, content, prescription_id,
                    author_name, author_email, created_at, updated_at, deleted_at
             FROM pills WHERE id = ?1",
            params![resolved_id],
            row_to_pill,
        )
        .context("failed to read revised pill")?;

    tx.commit()?;
    Ok(Some(pill))
}

/// Soft delete de una pill.
///
/// Acepta UUID completo o prefijo ≥8 chars. Devuelve `None` si no existe
/// o ya estaba descartada.
pub fn discard(conn: &mut Connection, id: &str) -> Result<Option<PillDiscardResult>> {
    let Some(resolved_id) = resolve_id(conn, "pills", id)? else {
        return Ok(None);
    };

    let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;

    let affected = tx
        .execute(
            "UPDATE pills SET deleted_at = datetime('now')
         WHERE id = ?1 AND deleted_at IS NULL",
            params![resolved_id],
        )
        .context("failed to discard pill")?;

    if affected == 0 {
        return Ok(None);
    }

    let deleted_at: String = tx.query_row(
        "SELECT deleted_at FROM pills WHERE id = ?1",
        params![resolved_id],
        |row| row.get(0),
    )?;

    tx.commit()?;
    Ok(Some(PillDiscardResult { id: resolved_id, deleted_at }))
}

/// Hard delete de una pill (irreversible).
///
/// Elimina la fila de `pills` permanentemente.
/// Devuelve `Some(id)` si se eliminó, `None` si la pill no existía.
pub fn hard_delete(conn: &mut Connection, id: &str) -> Result<Option<String>> {
    let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;

    let exists: bool = tx.query_row(
        "SELECT EXISTS(SELECT 1 FROM pills WHERE id = ?1)",
        params![id],
        |row| row.get(0),
    )?;

    if !exists {
        return Ok(None);
    }

    tx.execute("DELETE FROM pills WHERE id = ?1", params![id])
        .context("failed to delete pill")?;

    tx.commit()?;
    Ok(Some(id.to_string()))
}

/// Lista todas las pills de una prescription, ordenadas por fecha de creación.
pub fn list_by_prescription(conn: &Connection, prescription_id: &str) -> Result<Vec<Pill>> {
    let mut stmt = conn.prepare(
        "SELECT id, compound, title, content, prescription_id,
                author_name, author_email, created_at, updated_at, deleted_at
         FROM pills
         WHERE prescription_id = ?1
         ORDER BY created_at ASC, id ASC",
    )?;
    let pills = stmt
        .query_map(params![prescription_id], row_to_pill)?
        .collect::<rusqlite::Result<Vec<_>>>()
        .context("failed to list pills for prescription")?;
    Ok(pills)
}

/// Cuenta el total de pills activas en un bottle (todas sus prescriptions).
pub fn count_by_bottle(conn: &Connection, bottle_id: &str) -> Result<u32> {
    conn.query_row(
        "SELECT COUNT(p.id) FROM pills p
         JOIN prescriptions rx ON rx.id = p.prescription_id
         WHERE rx.bottle_id = ?1 AND rx.deleted_at IS NULL AND p.deleted_at IS NULL",
        params![bottle_id],
        |r| r.get(0),
    )
    .context("failed to count pills by bottle")
}

/// Pills más recientes de un bottle, ordenadas de más nueva a más antigua.
pub fn list_recent(conn: &Connection, bottle_id: &str, limit: u32) -> Result<Vec<Pill>> {
    let mut stmt = conn.prepare(
        "SELECT p.id, p.compound, p.title, p.content, p.prescription_id,
                p.author_name, p.author_email, p.created_at, p.updated_at, p.deleted_at
         FROM pills p
         JOIN prescriptions rx ON rx.id = p.prescription_id
         WHERE rx.bottle_id = ?1 AND rx.deleted_at IS NULL AND p.deleted_at IS NULL
         ORDER BY p.created_at DESC, p.id DESC
         LIMIT ?2",
    )?;
    let pills = stmt
        .query_map(params![bottle_id, limit], row_to_pill)?
        .collect::<rusqlite::Result<Vec<_>>>()
        .context("failed to list recent pills for bottle")?;
    Ok(pills)
}

// ─── Stats ────────────────────────────────────────────────────────────────────

/// Conteo de pills creadas en un día concreto.
#[derive(Debug, serde::Serialize)]
pub struct DayCount {
    pub date: String,
    pub count: i64,
}

/// Pills creadas por día en los últimos `days` días (incluyendo días con 0).
pub fn activity_by_day(conn: &Connection, days: u32) -> Result<Vec<DayCount>> {
    let days = days.max(1) as i64;
    let mut stmt = conn.prepare(
        "WITH RECURSIVE dates(d) AS (
             SELECT date('now', 'localtime', '-' || (?1 - 1) || ' days')
             UNION ALL
             SELECT date(d, '+1 day') FROM dates WHERE d < date('now', 'localtime')
         )
         SELECT d AS date,
                COUNT(p.id) AS count
         FROM dates
         LEFT JOIN pills p
             ON date(p.created_at, 'localtime') = d
             AND p.deleted_at IS NULL
         GROUP BY d
         ORDER BY d",
    )?;
    let rows = stmt
        .query_map(params![days], |row| {
            Ok(DayCount {
                date: row.get(0)?,
                count: row.get(1)?,
            })
        })?
        .collect::<rusqlite::Result<Vec<_>>>()
        .context("failed to query pill activity")?;
    Ok(rows)
}

/// Pills en la prescription actualmente abierta (0 si no hay ninguna).
pub fn open_rx_pill_count(conn: &Connection) -> Result<i64> {
    conn.query_row(
        "SELECT COUNT(*) FROM pills p
         JOIN prescriptions rx ON p.prescription_id = rx.id
         WHERE rx.ended_at IS NULL AND rx.deleted_at IS NULL
           AND p.deleted_at IS NULL",
        [],
        |row| row.get(0),
    )
    .context("failed to count pills in open rx")
}

/// Mapea una fila de SQLite al tipo [`Pill`].
fn row_to_pill(row: &rusqlite::Row<'_>) -> rusqlite::Result<Pill> {
    Ok(Pill {
        id: row.get(0)?,
        compound: row.get(1)?,
        title: row.get(2)?,
        content: row.get(3)?,
        prescription_id: row.get(4)?,
        author_name: row.get(5)?,
        author_email: row.get(6)?,
        created_at: row.get(7)?,
        updated_at: row.get(8)?,
        deleted_at: row.get(9)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::connection::open_in_memory;
    use crate::db::store::{bottles, prescriptions};
    use crate::domain::bottle::{BottleScope, NewBottle};
    use crate::domain::pill::PillPatch;
    use crate::domain::prescription::NewPrescription;

    fn setup(conn: &mut Connection) -> (String, String) {
        let bottle = bottles::create(
            conn,
            &NewBottle {
                name: "test".into(),
                display_name: "Test".into(),
                directory: "/tmp/test".into(),
                scope: BottleScope::Local,
            },
        )
        .unwrap();

        let rx = prescriptions::open(
            conn,
            &NewPrescription {
                bottle_id: bottle.id.clone(),
                title: "Sesión de prueba".into(),
                author_name: None,
                author_email: None,
            },
        )
        .unwrap();

        (bottle.id, rx.id)
    }

    fn sample_pill(rx_id: &str) -> NewPill {
        NewPill {
            title: "Decisión de diseño".into(),
            content: "Usamos UUID v7 para sync_id porque preserva el orden temporal.".into(),
            compound: "decision".into(),
            prescription_id: rx_id.to_string(),
            author_name: Some("Kevin".into()),
            author_email: None,
        }
    }

    #[test]
    fn take_resolves_12char_prescription_prefix() {
        let mut conn = open_in_memory().unwrap();
        let (_, rx_id) = setup(&mut conn);
        let short = rx_id.replace('-', "").chars().take(12).collect::<String>();

        let mut input = sample_pill(&rx_id);
        input.prescription_id = short;
        let result = take(&mut conn, &input).unwrap();
        assert_eq!(result.action, "created");

        // La pill debe quedar enlazada al UUID completo de la prescription
        let pill = read(&conn, &result.id).unwrap().unwrap();
        assert_eq!(pill.prescription_id, rx_id);
    }

    #[test]
    fn take_short_prescription_id_not_found() {
        let mut conn = open_in_memory().unwrap();
        let (_, _rx_id) = setup(&mut conn);

        let mut input = sample_pill("019dca5fc003");
        input.prescription_id = "019dca5fc003".into();
        let err = take(&mut conn, &input).unwrap_err();
        assert!(err.to_string().contains("prescription_required"));
    }

    #[test]
    fn take_and_read() {
        let mut conn = open_in_memory().unwrap();
        let (_, rx_id) = setup(&mut conn);

        let input = sample_pill(&rx_id);
        let result = take(&mut conn, &input).unwrap();
        assert_eq!(result.action, "created");
        assert!(!result.id.is_empty());
        assert_eq!(result.title, input.title);
        assert_eq!(result.compound, input.compound.as_str());
        assert_eq!(result.content, input.content);

        let pill = read(&conn, &result.id).unwrap().unwrap();
        assert_eq!(pill.compound, "decision");
        assert_eq!(pill.prescription_id, rx_id);
    }

    #[test]
    fn take_closed_prescription_fails() {
        let mut conn = open_in_memory().unwrap();
        let (_, rx_id) = setup(&mut conn);
        prescriptions::close(&mut conn, &rx_id).unwrap();

        let err = take(&mut conn, &sample_pill(&rx_id)).unwrap_err();
        assert!(err.to_string().contains("prescription_required"));
    }

    #[test]
    fn revise_partial_patch() {
        let mut conn = open_in_memory().unwrap();
        let (_, rx_id) = setup(&mut conn);

        let pill = take(&mut conn, &sample_pill(&rx_id)).unwrap();
        let updated = revise(
            &mut conn,
            &pill.id,
            &PillPatch {
                title: Some("Título revisado".into()),
                content: None,
                compound: None,
            },
        )
        .unwrap()
        .unwrap();

        assert_eq!(updated.title, "Título revisado");
        // content sin cambiar
        assert!(updated.content.contains("UUID v7"));
    }

    #[test]
    fn discard_soft_deletes() {
        let mut conn = open_in_memory().unwrap();
        let (_, rx_id) = setup(&mut conn);

        let pill = take(&mut conn, &sample_pill(&rx_id)).unwrap();
        let result = discard(&mut conn, &pill.id).unwrap().unwrap();
        assert_eq!(result.id, pill.id);
        assert!(!result.deleted_at.is_empty());

        // read ya no lo devuelve
        assert!(read(&conn, &pill.id).unwrap().is_none());
    }

    #[test]
    fn discard_twice_returns_none() {
        let mut conn = open_in_memory().unwrap();
        let (_, rx_id) = setup(&mut conn);
        let pill = take(&mut conn, &sample_pill(&rx_id)).unwrap();

        discard(&mut conn, &pill.id).unwrap();
        assert!(discard(&mut conn, &pill.id).unwrap().is_none());
    }

    #[test]
    fn read_missing_returns_none() {
        let conn = open_in_memory().unwrap();
        assert!(read(&conn, "00000000-0000-0000-0000-000000000000").unwrap().is_none());
    }

    #[test]
    fn revise_missing_returns_none() {
        let mut conn = open_in_memory().unwrap();
        let result = revise(
            &mut conn,
            "00000000-0000-0000-0000-000000000000",
            &PillPatch {
                title: Some("x".into()),
                content: None,
                compound: None,
            },
        )
        .unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn revise_discarded_returns_none() {
        let mut conn = open_in_memory().unwrap();
        let (_, rx_id) = setup(&mut conn);
        let pill = take(&mut conn, &sample_pill(&rx_id)).unwrap();
        discard(&mut conn, &pill.id).unwrap();

        let result = revise(
            &mut conn,
            &pill.id,
            &PillPatch {
                title: Some("nuevo".into()),
                content: None,
                compound: None,
            },
        )
        .unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn list_by_prescription_returns_ordered_pills() {
        let mut conn = open_in_memory().unwrap();
        let (_, rx_id) = setup(&mut conn);

        take(&mut conn, &sample_pill(&rx_id)).unwrap();
        take(
            &mut conn,
            &NewPill {
                title: "Segunda pill".into(),
                content: "Contenido de la segunda.".into(),
                compound: "discovery".into(),
                prescription_id: rx_id.clone(),
                author_name: None,
                author_email: None,
            },
        )
        .unwrap();

        let pills = list_by_prescription(&conn, &rx_id).unwrap();
        assert_eq!(pills.len(), 2);
        assert_eq!(pills[0].title, "Decisión de diseño");
        assert_eq!(pills[1].title, "Segunda pill");
    }

    #[test]
    fn list_by_prescription_empty_for_unknown() {
        let conn = open_in_memory().unwrap();
        let pills = list_by_prescription(&conn, "rx-inexistente").unwrap();
        assert!(pills.is_empty());
    }

    #[test]
    fn hard_delete_pill_removes_row() {
        let mut conn = open_in_memory().unwrap();
        let (_, rx_id) = setup(&mut conn);

        let result = take(&mut conn, &sample_pill(&rx_id)).unwrap();
        let pill_id = result.id.clone();

        let deleted = hard_delete(&mut conn, &pill_id).unwrap();
        assert_eq!(deleted, Some(pill_id.clone()));

        // pill ya no existe
        let pill_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM pills WHERE id = ?1", params![pill_id], |r| r.get(0))
            .unwrap();
        assert_eq!(pill_count, 0);
    }

    #[test]
    fn hard_delete_nonexistent_pill_returns_none() {
        let mut conn = open_in_memory().unwrap();
        let result = hard_delete(&mut conn, "00000000-0000-0000-0000-000000000000").unwrap();
        assert_eq!(result, None);
    }

    #[test]
    fn read_excludes_discarded() {
        let mut conn = open_in_memory().unwrap();
        let (_, rx_id) = setup(&mut conn);
        let pill = take(&mut conn, &sample_pill(&rx_id)).unwrap();
        discard(&mut conn, &pill.id).unwrap();

        // read() should return None for a discarded pill
        assert!(read(&conn, &pill.id).unwrap().is_none());
    }

    #[test]
    fn list_by_prescription_includes_discarded() {
        let mut conn = open_in_memory().unwrap();
        let (_, rx_id) = setup(&mut conn);
        let pill = take(&mut conn, &sample_pill(&rx_id)).unwrap();
        discard(&mut conn, &pill.id).unwrap();

        // list_by_prescription() should now include discarded pills
        let pills = list_by_prescription(&conn, &rx_id).unwrap();
        assert_eq!(pills.len(), 1);
        assert!(pills[0].deleted_at.is_some());
    }

    #[test]
    fn list_by_prescription_active_and_archived_together() {
        let mut conn = open_in_memory().unwrap();
        let (_, rx_id) = setup(&mut conn);

        // Active pill
        take(&mut conn, &sample_pill(&rx_id)).unwrap();

        // Discarded pill
        let pill2 = take(
            &mut conn,
            &NewPill {
                title: "Segunda pill".into(),
                content: "Contenido.".into(),
                compound: "discovery".into(),
                prescription_id: rx_id.clone(),
                author_name: None,
                author_email: None,
            },
        )
        .unwrap();
        discard(&mut conn, &pill2.id).unwrap();

        let pills = list_by_prescription(&conn, &rx_id).unwrap();
        assert_eq!(pills.len(), 2);

        let active_count = pills.iter().filter(|p| p.deleted_at.is_none()).count();
        let archived_count = pills.iter().filter(|p| p.deleted_at.is_some()).count();
        assert_eq!(active_count, 1);
        assert_eq!(archived_count, 1);
    }

    #[test]
    fn read_any_returns_archived_pill() {
        let mut conn = open_in_memory().unwrap();
        let (_, rx_id) = setup(&mut conn);
        let pill = take(&mut conn, &sample_pill(&rx_id)).unwrap();
        discard(&mut conn, &pill.id).unwrap();

        let found = read_any(&conn, &pill.id).unwrap();
        assert!(found.is_some());
        assert!(found.unwrap().deleted_at.is_some());
    }

    #[test]
    fn read_excludes_archived_but_read_any_does_not() {
        let mut conn = open_in_memory().unwrap();
        let (_, rx_id) = setup(&mut conn);
        let pill = take(&mut conn, &sample_pill(&rx_id)).unwrap();
        discard(&mut conn, &pill.id).unwrap();

        assert!(read(&conn, &pill.id).unwrap().is_none());
        assert!(read_any(&conn, &pill.id).unwrap().is_some());
    }

    #[test]
    fn read_by_12char_prefix() {
        let mut conn = open_in_memory().unwrap();
        let (_, rx_id) = setup(&mut conn);
        let result = take(&mut conn, &sample_pill(&rx_id)).unwrap();
        let short = result.id.replace('-', "").chars().take(12).collect::<String>();
        let found = read(&conn, &short).unwrap().unwrap();
        assert_eq!(found.id, result.id);
    }

    #[test]
    fn revise_by_short_id() {
        let mut conn = open_in_memory().unwrap();
        let (_, rx_id) = setup(&mut conn);
        let result = take(&mut conn, &sample_pill(&rx_id)).unwrap();
        let short = result.id.replace('-', "").chars().take(12).collect::<String>();
        let updated = revise(
            &mut conn,
            &short,
            &PillPatch { title: Some("Revisada por prefijo".into()), content: None, compound: None },
        )
        .unwrap()
        .unwrap();
        assert_eq!(updated.title, "Revisada por prefijo");
    }

    #[test]
    fn discard_by_short_id() {
        let mut conn = open_in_memory().unwrap();
        let (_, rx_id) = setup(&mut conn);
        let result = take(&mut conn, &sample_pill(&rx_id)).unwrap();
        let short = result.id.replace('-', "").chars().take(12).collect::<String>();
        let discarded = discard(&mut conn, &short).unwrap().unwrap();
        assert_eq!(discarded.id, result.id);
        assert!(read(&conn, &result.id).unwrap().is_none());
    }

    #[test]
    fn read_too_short_returns_invalid_id() {
        let conn = open_in_memory().unwrap();
        let err = read(&conn, "abc").unwrap_err();
        let typed = err.downcast_ref::<PillboxError>().unwrap();
        assert!(matches!(typed, PillboxError::InvalidId { .. }));
    }

    #[test]
    fn read_ambiguous_returns_ambiguous_id() {
        let mut conn = open_in_memory().unwrap();
        let (_, rx_id) = setup(&mut conn);
        conn.execute(
            "INSERT INTO pills (id, compound, title, content, prescription_id)
             VALUES ('01234567-aaaa-7000-8000-000000000001', 'decision', 'A', 'c', ?1)",
            params![rx_id],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO pills (id, compound, title, content, prescription_id)
             VALUES ('01234567-aaaa-7000-8000-000000000002', 'decision', 'B', 'c', ?1)",
            params![rx_id],
        )
        .unwrap();
        let err = read(&conn, "01234567aaaa").unwrap_err();
        let typed = err.downcast_ref::<PillboxError>().unwrap();
        assert!(matches!(typed, PillboxError::AmbiguousId { .. }));
    }
}
