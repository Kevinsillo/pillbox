use anyhow::{Context, Result};
use rusqlite::{params, Connection, TransactionBehavior};
use serde::Serialize;
use uuid::Uuid;

use crate::domain::pill::{NewPill, Pill, PillPatch};

// ─── Tipos de resultado ───────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct PillTakeResult {
    pub id: i64,
    pub sync_id: String,
    pub action: &'static str, // "created"
}

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
pub fn take(conn: &mut Connection, input: &NewPill) -> Result<PillTakeResult> {
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
        .context("error al verificar la prescription")?;

    if !rx_open {
        anyhow::bail!(
            "prescription_required: la prescription '{}' no existe o está cerrada",
            input.prescription_id
        );
    }

    let sync_id = Uuid::now_v7().to_string();
    let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;

    tx.execute(
        "INSERT INTO pills
             (sync_id, compound, title, content, prescription_id, dispenser, author_name, author_email)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        params![
            sync_id,
            input.compound.as_str(),
            input.title,
            input.content,
            input.prescription_id,
            input.dispenser,
            input.author_name,
            input.author_email,
        ],
    )
    .context("no se pudo insertar la pill")?;

    let id = tx.last_insert_rowid();

    tx.execute(
        "INSERT INTO dispense_log (prescription_id, action, pill_id)
         VALUES (?1, 'pill_take', ?2)",
        params![input.prescription_id, id],
    )?;

    tx.commit()?;
    Ok(PillTakeResult {
        id,
        sync_id,
        action: "created",
    })
}

/// Lee una pill completa por ID (no descartada).
pub fn read(conn: &Connection, id: i64) -> Result<Option<Pill>> {
    match conn.query_row(
        "SELECT id, sync_id, compound, title, content, prescription_id,
                dispenser, author_name, author_email, created_at, updated_at
         FROM pills WHERE id = ?1 AND deleted_at IS NULL",
        params![id],
        row_to_pill,
    ) {
        Ok(p) => Ok(Some(p)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(e).context("no se pudo leer la pill"),
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
        .context("no se pudo actualizar la pill")?;

    if affected == 0 {
        return Ok(None);
    }

    // Log de la revisión
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
                    dispenser, author_name, author_email, created_at, updated_at
             FROM pills WHERE id = ?1",
            params![id],
            row_to_pill,
        )
        .context("no se pudo leer la pill revisada")?;

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
        .context("no se pudo descartar la pill")?;

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

/// Lista todas las pills de una prescription, ordenadas por fecha de creación.
pub fn list_by_prescription(conn: &Connection, prescription_id: &str) -> Result<Vec<Pill>> {
    let mut stmt = conn.prepare(
        "SELECT id, sync_id, compound, title, content, prescription_id,
                dispenser, author_name, author_email, created_at, updated_at
         FROM pills
         WHERE prescription_id = ?1 AND deleted_at IS NULL
         ORDER BY created_at ASC",
    )?;
    let pills = stmt
        .query_map(params![prescription_id], row_to_pill)?
        .collect::<rusqlite::Result<Vec<_>>>()
        .context("no se pudo listar las pills de la prescription")?;
    Ok(pills)
}

fn row_to_pill(row: &rusqlite::Row<'_>) -> rusqlite::Result<Pill> {
    Ok(Pill {
        id: row.get(0)?,
        sync_id: row.get(1)?,
        compound: row.get(2)?,
        title: row.get(3)?,
        content: row.get(4)?,
        prescription_id: row.get(5)?,
        dispenser: row.get(6)?,
        author_name: row.get(7)?,
        author_email: row.get(8)?,
        created_at: row.get(9)?,
        updated_at: row.get(10)?,
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

    fn setup(conn: &mut Connection) -> (i64, String) {
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
                bottle_id: bottle.id,
                title: "Sesión de prueba".into(),
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
            dispenser: Some("pill_take".into()),
            author_name: Some("Kevin".into()),
            author_email: None,
        }
    }

    #[test]
    fn take_and_read() {
        let mut conn = open_in_memory().unwrap();
        let (_, rx_id) = setup(&mut conn);

        let result = take(&mut conn, &sample_pill(&rx_id)).unwrap();
        assert_eq!(result.action, "created");
        assert!(result.id > 0);

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
}
