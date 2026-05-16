//! Operaciones de store para la entidad [`Pill`].

use anyhow::{Context, Result};
use rusqlite::{params, Connection, TransactionBehavior};
use serde::Serialize;
use uuid::Uuid;

use crate::db::store::id_resolver::resolve_id;
use crate::db::store::prescriptions;
use crate::domain::pill::{NewPill, Pill, PillPatch};
use crate::domain::{Paginated, PaginationParams};
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

    // Si el id resuelve pero la prescription está cerrada o descartada → PrescriptionClosed.
    // Si la prescription no existe físicamente → PrescriptionRequired (no pudo resolverse a
    // una fila real). Reusamos ensure_rx_open que distingue ambos casos.
    prescriptions::ensure_rx_open(conn, &resolved_rx_id).map_err(|e| {
        match e.downcast::<PillboxError>() {
            Ok(PillboxError::PrescriptionNotFound { .. }) => {
                anyhow::Error::from(PillboxError::PrescriptionRequired {
                    prescription_id: input.prescription_id.clone(),
                })
            }
            Ok(other) => anyhow::Error::from(other),
            Err(e) => e,
        }
    })?;

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

    // Guard: la prescription padre debe estar abierta antes de modificar la pill.
    let rx_id: Option<String> = match conn.query_row(
        "SELECT prescription_id FROM pills WHERE id = ?1 AND deleted_at IS NULL",
        params![resolved_id],
        |row| row.get::<_, String>(0),
    ) {
        Ok(rx) => Some(rx),
        Err(rusqlite::Error::QueryReturnedNoRows) => None,
        Err(e) => return Err(e).context("failed to read pill's prescription_id"),
    };
    let Some(rx_id) = rx_id else {
        return Ok(None);
    };
    prescriptions::ensure_rx_open(conn, &rx_id)?;

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

    // Guard: la prescription padre debe estar abierta antes de descartar la pill.
    let rx_id: Option<String> = match conn.query_row(
        "SELECT prescription_id FROM pills WHERE id = ?1 AND deleted_at IS NULL",
        params![resolved_id],
        |row| row.get::<_, String>(0),
    ) {
        Ok(rx) => Some(rx),
        Err(rusqlite::Error::QueryReturnedNoRows) => None,
        Err(e) => return Err(e).context("failed to read pill's prescription_id"),
    };
    let Some(rx_id) = rx_id else {
        return Ok(None);
    };
    prescriptions::ensure_rx_open(conn, &rx_id)?;

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

/// Lista pills activas de una prescription (paginado, orden `created_at ASC`).
pub fn list_by_prescription(
    conn: &Connection,
    prescription_id: &str,
    pagination: &PaginationParams,
) -> Result<Paginated<Pill>> {
    let total: u64 = conn.query_row(
        "SELECT COUNT(*) FROM pills
         WHERE prescription_id = ?1 AND deleted_at IS NULL",
        params![prescription_id],
        |r| r.get(0),
    )?;

    let limit = pagination.limit() as i64;
    let offset = pagination.offset() as i64;

    let mut stmt = conn.prepare(
        "SELECT id, compound, title, content, prescription_id,
                author_name, author_email, created_at, updated_at, deleted_at
         FROM pills
         WHERE prescription_id = ?1 AND deleted_at IS NULL
         ORDER BY created_at ASC, id ASC
         LIMIT ?2 OFFSET ?3",
    )?;
    let items = stmt
        .query_map(params![prescription_id, limit, offset], row_to_pill)?
        .collect::<rusqlite::Result<Vec<_>>>()
        .context("failed to list pills for prescription")?;

    Ok(Paginated {
        items,
        total,
        page: pagination.page,
        page_size: pagination.page_size,
    })
}

/// Cuenta las pills activas (no descartadas) de una prescription.
///
/// El caller debe pasar el `prescription_id` ya resuelto (UUID completo).
pub fn count_active_by_prescription(conn: &Connection, prescription_id: &str) -> Result<u64> {
    let n: u64 = conn.query_row(
        "SELECT COUNT(*) FROM pills
         WHERE prescription_id = ?1 AND deleted_at IS NULL",
        params![prescription_id],
        |r| r.get(0),
    )?;
    Ok(n)
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

/// Compounds distintos usados en pills, ordenados por frecuencia descendente.
///
/// Si `bottle_id` es `Some`, filtra a pills cuya prescription pertenece a ese
/// bottle (acepta UUID exacto). Si es `None`, agrega sobre toda la DB.
/// Devuelve `(compound, count)` ordenado por count DESC y compound ASC como
/// desempate estable.
pub fn distinct_compounds(
    conn: &Connection,
    bottle_id: Option<&str>,
    limit: u32,
) -> Result<Vec<(String, i64)>> {
    let mut stmt = conn.prepare(
        "SELECT p.compound, COUNT(*) as c
         FROM pills p
         JOIN prescriptions rx ON p.prescription_id = rx.id
         WHERE p.deleted_at IS NULL
           AND (?1 IS NULL OR rx.bottle_id = ?1)
         GROUP BY p.compound
         ORDER BY c DESC, p.compound ASC
         LIMIT ?2",
    )?;
    let rows = stmt
        .query_map(params![bottle_id, limit], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
        })?
        .collect::<rusqlite::Result<Vec<_>>>()
        .context("failed to load distinct pill compounds")?;
    Ok(rows)
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
        // Una prescription existente pero cerrada → prescription_closed
        // (unificado con la guard de revise/discard).
        assert!(err.to_string().contains("prescription_closed"));
    }

    #[test]
    fn pill_revise_blocked_on_closed_prescription() {
        let mut conn = open_in_memory().unwrap();
        let (_, rx_id) = setup(&mut conn);

        // Crear pill en rx abierta
        let pill = take(&mut conn, &sample_pill(&rx_id)).unwrap();
        let original_title = pill.title.clone();

        // Cerrar la rx
        prescriptions::close(&mut conn, &rx_id).unwrap();

        // Intentar revisar
        let err = revise(
            &mut conn,
            &pill.id,
            &PillPatch {
                title: Some("Nuevo título".into()),
                content: None,
                compound: None,
            },
        )
        .unwrap_err();
        let typed = err.downcast_ref::<PillboxError>().unwrap();
        match typed {
            PillboxError::PrescriptionClosed { prescription_id } => {
                assert_eq!(prescription_id, &rx_id);
            }
            other => panic!("expected PrescriptionClosed, got {:?}", other),
        }

        // Verificar que la pill NO fue modificada
        let still = read_any(&conn, &pill.id).unwrap().unwrap();
        assert_eq!(still.title, original_title);
    }

    #[test]
    fn pill_discard_blocked_on_closed_prescription() {
        let mut conn = open_in_memory().unwrap();
        let (_, rx_id) = setup(&mut conn);

        let pill = take(&mut conn, &sample_pill(&rx_id)).unwrap();

        prescriptions::close(&mut conn, &rx_id).unwrap();

        let err = discard(&mut conn, &pill.id).unwrap_err();
        let typed = err.downcast_ref::<PillboxError>().unwrap();
        match typed {
            PillboxError::PrescriptionClosed { prescription_id } => {
                assert_eq!(prescription_id, &rx_id);
            }
            other => panic!("expected PrescriptionClosed, got {:?}", other),
        }

        // Verificar que la pill NO fue descartada
        let still = read(&conn, &pill.id).unwrap().unwrap();
        assert!(still.deleted_at.is_none());
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

        let pills = list_by_prescription(&conn, &rx_id, &PaginationParams::default()).unwrap();
        assert_eq!(pills.items.len(), 2);
        assert_eq!(pills.items[0].title, "Decisión de diseño");
        assert_eq!(pills.items[1].title, "Segunda pill");
    }

    #[test]
    fn list_by_prescription_empty_for_unknown() {
        let conn = open_in_memory().unwrap();
        let pills = list_by_prescription(&conn, "rx-inexistente", &PaginationParams::default()).unwrap();
        assert!(pills.items.is_empty());
        assert_eq!(pills.total, 0);
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
    fn list_by_prescription_excludes_discarded() {
        let mut conn = open_in_memory().unwrap();
        let (_, rx_id) = setup(&mut conn);
        let pill = take(&mut conn, &sample_pill(&rx_id)).unwrap();
        discard(&mut conn, &pill.id).unwrap();

        let pills = list_by_prescription(&conn, &rx_id, &PaginationParams::default()).unwrap();
        assert!(pills.items.is_empty());
        assert_eq!(pills.total, 0);
    }

    #[test]
    fn list_by_prescription_only_active() {
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

        let pills = list_by_prescription(&conn, &rx_id, &PaginationParams::default()).unwrap();
        assert_eq!(pills.items.len(), 1);
        assert_eq!(pills.total, 1);
        assert!(pills.items.iter().all(|p| p.deleted_at.is_none()));
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
    fn distinct_compounds_orders_by_count_desc_then_compound_asc() {
        let mut conn = open_in_memory().unwrap();
        let (bottle_id_1, rx_id_1) = setup(&mut conn);

        // bottle 1: 2x "alpha", 1x "beta"
        for (title, compound) in [
            ("p1", "alpha"),
            ("p2", "alpha"),
            ("p3", "beta"),
        ] {
            take(
                &mut conn,
                &NewPill {
                    title: title.into(),
                    content: "c".into(),
                    compound: compound.into(),
                    prescription_id: rx_id_1.clone(),
                    author_name: None,
                    author_email: None,
                },
            )
            .unwrap();
        }

        // bottle 2: 1x "alpha", 1x "gamma" (gamma will be soft-deleted)
        let bottle_2 = bottles::create(
            &mut conn,
            &NewBottle {
                name: "other".into(),
                display_name: "Other".into(),
                directory: "/tmp/other".into(),
                scope: BottleScope::Local,
            },
        )
        .unwrap();
        let rx_2 = prescriptions::open(
            &mut conn,
            &NewPrescription {
                bottle_id: bottle_2.id.clone(),
                title: "rx2".into(),
                author_name: None,
                author_email: None,
            },
        )
        .unwrap();
        take(
            &mut conn,
            &NewPill {
                title: "p4".into(),
                content: "c".into(),
                compound: "alpha".into(),
                prescription_id: rx_2.id.clone(),
                author_name: None,
                author_email: None,
            },
        )
        .unwrap();
        let gamma = take(
            &mut conn,
            &NewPill {
                title: "p5".into(),
                content: "c".into(),
                compound: "gamma".into(),
                prescription_id: rx_2.id.clone(),
                author_name: None,
                author_email: None,
            },
        )
        .unwrap();
        discard(&mut conn, &gamma.id).unwrap(); // gamma excluded

        // Sin filtro: alpha=3 (cross-bottle), beta=1. gamma ausente.
        let rows = distinct_compounds(&conn, None, 50).unwrap();
        let alpha = rows.iter().find(|(c, _)| c == "alpha").unwrap();
        let beta = rows.iter().find(|(c, _)| c == "beta").unwrap();
        assert_eq!(alpha.1, 3);
        assert_eq!(beta.1, 1);
        assert!(!rows.iter().any(|(c, _)| c == "gamma"));
        // alpha (count 3) antes que beta (count 1)
        let i_alpha = rows.iter().position(|(c, _)| c == "alpha").unwrap();
        let i_beta = rows.iter().position(|(c, _)| c == "beta").unwrap();
        assert!(i_alpha < i_beta);

        // Filtro por bottle_id_1: alpha=2, beta=1 solamente.
        let rows_b1 = distinct_compounds(&conn, Some(&bottle_id_1), 50).unwrap();
        let alpha_b1 = rows_b1.iter().find(|(c, _)| c == "alpha").unwrap();
        let beta_b1 = rows_b1.iter().find(|(c, _)| c == "beta").unwrap();
        assert_eq!(alpha_b1.1, 2);
        assert_eq!(beta_b1.1, 1);
        // No incluye compounds del otro bottle (sólo alpha estaba allá, sumando 1)
        assert_eq!(rows_b1.iter().map(|(_, n)| *n).sum::<i64>(), 3);
    }

    #[test]
    fn distinct_compounds_tiebreak_alphabetical() {
        let mut conn = open_in_memory().unwrap();
        let (_, rx_id) = setup(&mut conn);

        // Dos compounds con mismo count → alfabético ASC.
        for compound in ["zebra", "apple", "zebra", "apple"] {
            take(
                &mut conn,
                &NewPill {
                    title: "t".into(),
                    content: "c".into(),
                    compound: compound.into(),
                    prescription_id: rx_id.clone(),
                    author_name: None,
                    author_email: None,
                },
            )
            .unwrap();
        }

        let rows = distinct_compounds(&conn, None, 50).unwrap();
        let i_apple = rows.iter().position(|(c, _)| c == "apple").unwrap();
        let i_zebra = rows.iter().position(|(c, _)| c == "zebra").unwrap();
        assert!(i_apple < i_zebra, "apple debe ir antes que zebra en tiebreak ASC");
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
