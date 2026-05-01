//! Operaciones de store para la entidad [`Pill`].

use anyhow::{Context, Result};
use rusqlite::{params, Connection, TransactionBehavior};
use serde::Serialize;
use uuid::Uuid;

use crate::domain::pill::{NewPill, Pill, PillPatch};
use crate::error::PillboxError;

// ─── Tipos de resultado ───────────────────────────────────────────────────────

/// Resultado de guardar una pill nueva.
#[derive(Debug, Serialize)]
pub struct PillStoreResult {
    pub id: i64,
    pub sync_id: String,
    pub action: &'static str, // "created"
    pub title: String,
    pub compound: String,
    pub content: String,
}

/// Resultado de descartar una pill (soft delete).
#[derive(Debug, Serialize)]
pub struct PillDiscardResult {
    pub id: i64,
    pub deleted_at: String,
}

// ─── Store ────────────────────────────────────────────────────────────────────

/// Guarda una pill nueva en la prescription activa.
///
/// La prescription debe existir y estar abierta (`ended_at IS NULL`).
/// Falla con `prescription_required` si no se cumple.
pub fn take(conn: &mut Connection, input: &NewPill) -> Result<PillStoreResult> {
    // Verificar que la prescription existe y está abierta (fuera de tx: lectura rápida)
    let rx_open: bool = conn
        .query_row(
            "SELECT EXISTS(
             SELECT 1 FROM prescriptions
             WHERE id = ?1 AND ended_at IS NULL AND deleted_at IS NULL
         )",
            params![input.prescription_id],
            |row| row.get(0),
        )
        .context("failed to check prescription status")?;

    if !rx_open {
        return Err(PillboxError::PrescriptionRequired {
            prescription_id: input.prescription_id.clone(),
        }
        .into());
    }

    let sync_id = Uuid::now_v7().to_string();
    let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;

    tx.execute(
        "INSERT INTO pills
             (sync_id, compound, title, content, prescription_id, author_name, author_email)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![
            sync_id,
            input.compound.as_str(),
            input.title,
            input.content,
            input.prescription_id,
            input.author_name,
            input.author_email,
        ],
    )
    .context("failed to insert pill")?;

    let id = tx.last_insert_rowid();

    tx.execute(
        "INSERT INTO dispense_log (prescription_id, action, pill_id)
         VALUES (?1, 'pill_store', ?2)",
        params![input.prescription_id, id],
    )?;

    tx.commit()?;
    Ok(PillStoreResult {
        id,
        sync_id,
        action: "created",
        title: input.title.clone(),
        compound: input.compound.as_str().to_string(),
        content: input.content.clone(),
    })
}

/// Lee una pill completa por ID numérico (no descartada).
pub fn read(conn: &Connection, id: i64) -> Result<Option<Pill>> {
    match conn.query_row(
        "SELECT id, sync_id, compound, title, content, prescription_id,
                author_name, author_email, created_at, updated_at, deleted_at
         FROM pills WHERE id = ?1 AND deleted_at IS NULL",
        params![id],
        row_to_pill,
    ) {
        Ok(p) => Ok(Some(p)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(e).context("failed to read pill"),
    }
}

/// Lee una pill completa por ID numérico, incluyendo descartadas.
pub fn read_any(conn: &Connection, id: i64) -> Result<Option<Pill>> {
    match conn.query_row(
        "SELECT id, sync_id, compound, title, content, prescription_id,
                author_name, author_email, created_at, updated_at, deleted_at
         FROM pills WHERE id = ?1",
        params![id],
        row_to_pill,
    ) {
        Ok(p) => Ok(Some(p)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(e).context("failed to read pill"),
    }
}

/// Actualiza campos de una pill existente (patch parcial).
/// Solo se modifican los campos no-None. Devuelve `None` si no existe o fue descartada.
pub fn revise(conn: &mut Connection, id: i64, patch: &PillPatch) -> Result<Option<Pill>> {
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
                id,
            ],
        )
        .context("failed to update pill")?;

    if affected == 0 {
        return Ok(None);
    }

    let rx_id: Option<String> = tx
        .query_row(
            "SELECT prescription_id FROM pills WHERE id = ?1",
            params![id],
            |row| row.get(0),
        )
        .ok();

    if let Some(rx_id) = rx_id {
        tx.execute(
            "INSERT INTO dispense_log (prescription_id, action, pill_id)
             VALUES (?1, 'pill_revise', ?2)",
            params![rx_id, id],
        )?;
    }

    let pill = tx
        .query_row(
            "SELECT id, sync_id, compound, title, content, prescription_id,
                    author_name, author_email, created_at, updated_at, deleted_at
             FROM pills WHERE id = ?1",
            params![id],
            row_to_pill,
        )
        .context("failed to read revised pill")?;

    tx.commit()?;
    Ok(Some(pill))
}

/// Soft delete de una pill.
/// Devuelve `None` si no existe o ya estaba descartada.
pub fn discard(conn: &mut Connection, id: i64) -> Result<Option<PillDiscardResult>> {
    let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;

    let affected = tx
        .execute(
            "UPDATE pills SET deleted_at = datetime('now')
         WHERE id = ?1 AND deleted_at IS NULL",
            params![id],
        )
        .context("failed to discard pill")?;

    if affected == 0 {
        return Ok(None);
    }

    let deleted_at: String = tx.query_row(
        "SELECT deleted_at FROM pills WHERE id = ?1",
        params![id],
        |row| row.get(0),
    )?;

    let rx_id: Option<String> = tx
        .query_row(
            "SELECT prescription_id FROM pills WHERE id = ?1",
            params![id],
            |row| row.get(0),
        )
        .ok();

    if let Some(rx_id) = rx_id {
        tx.execute(
            "INSERT INTO dispense_log (prescription_id, action, pill_id)
             VALUES (?1, 'pill_discard', ?2)",
            params![rx_id, id],
        )?;
    }

    tx.commit()?;
    Ok(Some(PillDiscardResult { id, deleted_at }))
}

/// Hard delete de una pill (irreversible).
///
/// Elimina en orden: dispense_log WHERE pill_id=? → pill_links WHERE from_id=? OR to_id=? → pills WHERE id=?.
/// Devuelve `Some(id)` si se eliminó, `None` si la pill no existía.
pub fn hard_delete(conn: &mut Connection, id: i64) -> Result<Option<i64>> {
    let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;

    // Verificar que la pill existe
    let exists: bool = tx.query_row(
        "SELECT EXISTS(SELECT 1 FROM pills WHERE id = ?1)",
        params![id],
        |row| row.get(0),
    )?;

    if !exists {
        return Ok(None);
    }

    // 1. Eliminar entradas de dispense_log referenciando esta pill
    tx.execute(
        "DELETE FROM dispense_log WHERE pill_id = ?1",
        params![id],
    )
    .context("failed to delete dispense_log for pill")?;

    // 2. Eliminar pill_links donde esta pill participa
    tx.execute(
        "DELETE FROM pill_links WHERE from_id = ?1 OR to_id = ?1",
        params![id],
    )
    .context("failed to delete pill_links for pill")?;

    // 3. Eliminar la pill
    tx.execute("DELETE FROM pills WHERE id = ?1", params![id])
        .context("failed to delete pill")?;

    tx.commit()?;
    Ok(Some(id))
}

/// Lista todas las pills de una prescription, ordenadas por fecha de creación.
pub fn list_by_prescription(conn: &Connection, prescription_id: &str) -> Result<Vec<Pill>> {
    let mut stmt = conn.prepare(
        "SELECT id, sync_id, compound, title, content, prescription_id,
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
        sync_id: row.get(1)?,
        compound: row.get(2)?,
        title: row.get(3)?,
        content: row.get(4)?,
        prescription_id: row.get(5)?,
        author_name: row.get(6)?,
        author_email: row.get(7)?,
        created_at: row.get(8)?,
        updated_at: row.get(9)?,
        deleted_at: row.get(10)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::connection::open_in_memory;
    use crate::db::store::{bottles, prescriptions};
    use crate::domain::bottle::{BottleScope, NewBottle};
    use crate::domain::pill::{PillCompound, PillPatch};
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
            compound: PillCompound::Decision,
            prescription_id: rx_id.to_string(),
            author_name: Some("Kevin".into()),
            author_email: None,
        }
    }

    #[test]
    fn take_and_read() {
        let mut conn = open_in_memory().unwrap();
        let (_, rx_id) = setup(&mut conn);

        let input = sample_pill(&rx_id);
        let result = take(&mut conn, &input).unwrap();
        assert_eq!(result.action, "created");
        assert!(result.id > 0);
        assert_eq!(result.title, input.title);
        assert_eq!(result.compound, input.compound.as_str());
        assert_eq!(result.content, input.content);

        let pill = read(&conn, result.id).unwrap().unwrap();
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
            pill.id,
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
        let result = discard(&mut conn, pill.id).unwrap().unwrap();
        assert_eq!(result.id, pill.id);
        assert!(!result.deleted_at.is_empty());

        // read ya no lo devuelve
        assert!(read(&conn, pill.id).unwrap().is_none());
    }

    #[test]
    fn discard_twice_returns_none() {
        let mut conn = open_in_memory().unwrap();
        let (_, rx_id) = setup(&mut conn);
        let pill = take(&mut conn, &sample_pill(&rx_id)).unwrap();

        discard(&mut conn, pill.id).unwrap();
        assert!(discard(&mut conn, pill.id).unwrap().is_none());
    }

    #[test]
    fn read_missing_returns_none() {
        let conn = open_in_memory().unwrap();
        assert!(read(&conn, 9999).unwrap().is_none());
    }

    #[test]
    fn revise_missing_returns_none() {
        let mut conn = open_in_memory().unwrap();
        let result = revise(
            &mut conn,
            9999,
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
        discard(&mut conn, pill.id).unwrap();

        let result = revise(
            &mut conn,
            pill.id,
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
                compound: PillCompound::Discovery,
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
        let pill_id = result.id;

        // Insertar un pill_link self-referenciado para probar que se elimina
        conn.execute(
            "INSERT INTO pill_links (from_id, to_id, rel_type) VALUES (?1, ?1, 'related')",
            params![pill_id],
        )
        .unwrap();

        let deleted = hard_delete(&mut conn, pill_id).unwrap();
        assert_eq!(deleted, Some(pill_id));

        // pill ya no existe
        let pill_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM pills WHERE id = ?1", params![pill_id], |r| r.get(0))
            .unwrap();
        assert_eq!(pill_count, 0);

        // pill_links eliminados
        let link_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM pill_links WHERE from_id = ?1", params![pill_id], |r| r.get(0))
            .unwrap();
        assert_eq!(link_count, 0);

        // dispense_log de pill eliminado
        let log_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM dispense_log WHERE pill_id = ?1", params![pill_id], |r| r.get(0))
            .unwrap();
        assert_eq!(log_count, 0);
    }

    #[test]
    fn hard_delete_nonexistent_pill_returns_none() {
        let mut conn = open_in_memory().unwrap();
        let result = hard_delete(&mut conn, 9999).unwrap();
        assert_eq!(result, None);
    }

    #[test]
    fn read_excludes_discarded() {
        let mut conn = open_in_memory().unwrap();
        let (_, rx_id) = setup(&mut conn);
        let pill = take(&mut conn, &sample_pill(&rx_id)).unwrap();
        discard(&mut conn, pill.id).unwrap();

        // read() should return None for a discarded pill
        assert!(read(&conn, pill.id).unwrap().is_none());
    }

    #[test]
    fn list_by_prescription_includes_discarded() {
        let mut conn = open_in_memory().unwrap();
        let (_, rx_id) = setup(&mut conn);
        let pill = take(&mut conn, &sample_pill(&rx_id)).unwrap();
        discard(&mut conn, pill.id).unwrap();

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
                compound: PillCompound::Discovery,
                prescription_id: rx_id.clone(),
                author_name: None,
                author_email: None,
            },
        )
        .unwrap();
        discard(&mut conn, pill2.id).unwrap();

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
        discard(&mut conn, pill.id).unwrap();

        let found = read_any(&conn, pill.id).unwrap();
        assert!(found.is_some());
        assert!(found.unwrap().deleted_at.is_some());
    }

    #[test]
    fn read_excludes_archived_but_read_any_does_not() {
        let mut conn = open_in_memory().unwrap();
        let (_, rx_id) = setup(&mut conn);
        let pill = take(&mut conn, &sample_pill(&rx_id)).unwrap();
        discard(&mut conn, pill.id).unwrap();

        assert!(read(&conn, pill.id).unwrap().is_none());
        assert!(read_any(&conn, pill.id).unwrap().is_some());
    }
}
