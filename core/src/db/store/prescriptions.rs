use std::fmt;

use anyhow::{Context, Result};
use rusqlite::{params, Connection, TransactionBehavior};
use uuid::Uuid;

use crate::domain::prescription::{NewPrescription, Prescription};

// ─── Error tipado ─────────────────────────────────────────────────────────────

/// Error devuelto cuando ya existe una prescription abierta para el bottle.
///
/// Contiene suficiente información para que el modelo decida:
/// - Reutilizar la sesión activa pasando `id` a `pill_take`
/// - Cerrarla con `prescription_close` y abrir una nueva
#[derive(Debug, serde::Serialize)]
pub struct PrescriptionAlreadyOpen {
    pub id: String,
    pub title: String,
    pub started_at: String,
    pub pill_count: i64,
}

impl fmt::Display for PrescriptionAlreadyOpen {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "prescription_already_open: '{}' (id={}, iniciada={}, {} pills)",
            self.title, self.id, self.started_at, self.pill_count
        )
    }
}

impl std::error::Error for PrescriptionAlreadyOpen {}

// ─── Store ────────────────────────────────────────────────────────────────────

/// Abre una nueva prescription para el bottle dado.
///
/// Falla con `PrescriptionAlreadyOpen` si ya hay una prescription activa
/// (no cerrada ni descartada) para ese bottle.
pub fn open(conn: &mut Connection, input: &NewPrescription) -> Result<Prescription> {
    let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;

    // Verificar si ya hay una prescription abierta para este bottle
    let existing = match tx.query_row(
        "SELECT rx.id, rx.title, rx.started_at,
                (SELECT COUNT(*) FROM pills p
                 WHERE p.prescription_id = rx.id AND p.deleted_at IS NULL) AS pill_count
         FROM prescriptions rx
         WHERE rx.bottle_id = ?1
           AND rx.ended_at IS NULL
           AND rx.deleted_at IS NULL",
        params![input.bottle_id],
        |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, i64>(3)?,
            ))
        },
    ) {
        Ok(row) => Some(row),
        Err(rusqlite::Error::QueryReturnedNoRows) => None,
        Err(e) => return Err(e).context("error al verificar prescription abierta"),
    };

    if let Some((id, title, started_at, pill_count)) = existing {
        return Err(PrescriptionAlreadyOpen {
            id,
            title,
            started_at,
            pill_count,
        }
        .into());
    }

    // Verificar que el bottle existe
    let bottle_exists: bool = tx.query_row(
        "SELECT EXISTS(SELECT 1 FROM bottles WHERE id = ?1)",
        params![input.bottle_id],
        |row| row.get(0),
    )?;

    if !bottle_exists {
        anyhow::bail!("bottle_not_found: no existe el bottle {}", input.bottle_id);
    }

    let id = Uuid::now_v7().to_string();

    tx.execute(
        "INSERT INTO prescriptions (id, bottle_id, title) VALUES (?1, ?2, ?3)",
        params![id, input.bottle_id, input.title],
    )
    .context("no se pudo insertar la prescription")?;

    tx.execute(
        "INSERT INTO dispense_log (prescription_id, action) VALUES (?1, 'prescription_open')",
        params![id],
    )?;

    let prescription = tx
        .query_row(
            "SELECT id, bottle_id, title, started_at, ended_at, deleted_at
         FROM prescriptions WHERE id = ?1",
            params![id],
            row_to_prescription,
        )
        .context("no se pudo leer la prescription recién abierta")?;

    tx.commit()?;
    Ok(prescription)
}

/// Cierra una prescription existente (establece `ended_at`).
pub fn close(conn: &mut Connection, id: &str) -> Result<Prescription> {
    let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;

    let affected = tx
        .execute(
            "UPDATE prescriptions SET ended_at = datetime('now')
         WHERE id = ?1 AND ended_at IS NULL AND deleted_at IS NULL",
            params![id],
        )
        .context("no se pudo cerrar la prescription")?;

    if affected == 0 {
        anyhow::bail!("prescription_not_found_or_closed: {}", id);
    }

    tx.execute(
        "INSERT INTO dispense_log (prescription_id, action) VALUES (?1, 'prescription_close')",
        params![id],
    )?;

    let prescription = tx
        .query_row(
            "SELECT id, bottle_id, title, started_at, ended_at, deleted_at
         FROM prescriptions WHERE id = ?1",
            params![id],
            row_to_prescription,
        )
        .context("no se pudo leer la prescription cerrada")?;

    tx.commit()?;
    Ok(prescription)
}

/// Soft delete de una prescription y todas sus pills (cascade lógico).
pub fn discard(conn: &mut Connection, id: &str) -> Result<()> {
    let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;

    let affected = tx.execute(
        "UPDATE prescriptions SET deleted_at = datetime('now')
         WHERE id = ?1 AND deleted_at IS NULL",
        params![id],
    )?;

    if affected == 0 {
        anyhow::bail!("prescription_not_found: {}", id);
    }

    // Cascade soft delete de las pills de esta prescription
    tx.execute(
        "UPDATE pills SET deleted_at = datetime('now')
         WHERE prescription_id = ?1 AND deleted_at IS NULL",
        params![id],
    )?;

    tx.execute(
        "INSERT INTO dispense_log (prescription_id, action)
         VALUES (?1, 'prescription_discard')",
        params![id],
    )?;

    tx.commit()?;
    Ok(())
}

/// Lee una prescription por ID (no descartada).
pub fn read(conn: &Connection, id: &str) -> Result<Option<Prescription>> {
    match conn.query_row(
        "SELECT id, bottle_id, title, started_at, ended_at, deleted_at
         FROM prescriptions WHERE id = ?1 AND deleted_at IS NULL",
        params![id],
        row_to_prescription,
    ) {
        Ok(rx) => Ok(Some(rx)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(e).context("no se pudo leer la prescription"),
    }
}

/// Devuelve las últimas N prescriptions de un bottle (más recientes primero).
pub fn list_by_bottle(conn: &Connection, bottle_id: i64, limit: u32) -> Result<Vec<Prescription>> {
    let mut stmt = conn.prepare(
        "SELECT id, bottle_id, title, started_at, ended_at, deleted_at
         FROM prescriptions
         WHERE bottle_id = ?1 AND deleted_at IS NULL
         ORDER BY started_at DESC
         LIMIT ?2",
    )?;

    let rows = stmt
        .query_map(params![bottle_id, limit], row_to_prescription)?
        .collect::<rusqlite::Result<Vec<_>>>()
        .context("no se pudo listar las prescriptions")?;

    Ok(rows)
}

fn row_to_prescription(row: &rusqlite::Row<'_>) -> rusqlite::Result<Prescription> {
    Ok(Prescription {
        id: row.get(0)?,
        bottle_id: row.get(1)?,
        title: row.get(2)?,
        started_at: row.get(3)?,
        ended_at: row.get(4)?,
        deleted_at: row.get(5)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::connection::open_in_memory;
    use crate::db::store::bottles;
    use crate::domain::bottle::{BottleScope, NewBottle};

    fn make_bottle(conn: &mut Connection, name: &str) -> i64 {
        bottles::create(
            conn,
            &NewBottle {
                name: name.into(),
                display_name: name.into(),
                directory: format!("/tmp/{}", name),
                scope: BottleScope::Local,
            },
        )
        .unwrap()
        .id
    }

    #[test]
    fn open_and_close() {
        let mut conn = open_in_memory().unwrap();
        let bottle_id = make_bottle(&mut conn, "test");

        let rx = open(
            &mut conn,
            &NewPrescription {
                bottle_id,
                title: "Implementar auth".into(),
            },
        )
        .unwrap();

        assert_eq!(rx.bottle_id, bottle_id);
        assert!(rx.ended_at.is_none());

        let closed = close(&mut conn, &rx.id).unwrap();
        assert!(closed.ended_at.is_some());
    }

    #[test]
    fn collision_returns_typed_error() {
        let mut conn = open_in_memory().unwrap();
        let bottle_id = make_bottle(&mut conn, "colision");

        open(
            &mut conn,
            &NewPrescription {
                bottle_id,
                title: "Primera sesión".into(),
            },
        )
        .unwrap();

        let err = open(
            &mut conn,
            &NewPrescription {
                bottle_id,
                title: "Segunda sesión".into(),
            },
        )
        .unwrap_err();

        let already_open = err.downcast_ref::<PrescriptionAlreadyOpen>();
        assert!(already_open.is_some());
        assert_eq!(already_open.unwrap().title, "Primera sesión");
        assert_eq!(already_open.unwrap().pill_count, 0);
    }

    #[test]
    fn bottle_not_found_fails() {
        let mut conn = open_in_memory().unwrap();
        let err = open(
            &mut conn,
            &NewPrescription {
                bottle_id: 9999,
                title: "Sesión".into(),
            },
        )
        .unwrap_err();
        assert!(err.to_string().contains("bottle_not_found"));
    }

    #[test]
    fn close_nonexistent_fails() {
        let mut conn = open_in_memory().unwrap();
        assert!(close(&mut conn, "id-inexistente").is_err());
    }

    #[test]
    fn discard_cascades_to_pills() {
        let mut conn = open_in_memory().unwrap();
        let bottle_id = make_bottle(&mut conn, "cascade");

        let rx = open(
            &mut conn,
            &NewPrescription {
                bottle_id,
                title: "Sesión a descartar".into(),
            },
        )
        .unwrap();

        // Insertar una pill directamente para verificar el cascade
        conn.execute(
            "INSERT INTO pills (sync_id, compound, title, content, prescription_id)
             VALUES ('sync-001', 'decision', 'T', 'C', ?1)",
            params![rx.id],
        )
        .unwrap();

        discard(&mut conn, &rx.id).unwrap();

        let pill_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM pills WHERE prescription_id = ?1 AND deleted_at IS NULL",
                params![rx.id],
                |r| r.get(0),
            )
            .unwrap();

        assert_eq!(pill_count, 0);
    }
}
