use anyhow::{Context, Result};
use rusqlite::{params, Connection, TransactionBehavior};
use serde::Serialize;
use uuid::Uuid;

use crate::domain::capsule::{Capsule, CapsulePatch, NewCapsule};

// ─── Tipos de resultado ───────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct CapsuleTakeResult {
    pub id: i64,
    pub sync_id: String,
    pub action: &'static str, // "created"
}

#[derive(Debug, Serialize)]
pub struct CapsuleDiscardResult {
    pub id: i64,
    pub deleted_at: String,
}

// ─── Store ────────────────────────────────────────────────────────────────────

/// Guarda una capsule de conocimiento personal (sin prescription ni bottle).
pub fn take(conn: &mut Connection, input: &NewCapsule) -> Result<CapsuleTakeResult> {
    let sync_id = Uuid::now_v7().to_string();
    let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;

    tx.execute(
        "INSERT INTO capsules (sync_id, compound, title, content)
         VALUES (?1, ?2, ?3, ?4)",
        params![sync_id, input.compound.as_str(), input.title, input.content],
    )
    .context("no se pudo insertar la capsule")?;

    let id = tx.last_insert_rowid();

    tx.execute(
        "INSERT INTO dispense_log (action, pill_id) VALUES ('capsule_take', ?1)",
        params![id],
    )?;

    tx.commit()?;
    Ok(CapsuleTakeResult {
        id,
        sync_id,
        action: "created",
    })
}

/// Lee una capsule completa por ID (no descartada).
pub fn read(conn: &Connection, id: i64) -> Result<Option<Capsule>> {
    match conn.query_row(
        "SELECT id, sync_id, compound, title, content, created_at, updated_at
         FROM capsules WHERE id = ?1 AND deleted_at IS NULL",
        params![id],
        row_to_capsule,
    ) {
        Ok(c) => Ok(Some(c)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(e).context("no se pudo leer la capsule"),
    }
}

/// Actualiza campos de una capsule existente (patch parcial).
/// Devuelve `None` si no existe o fue descartada.
pub fn revise(conn: &mut Connection, id: i64, patch: &CapsulePatch) -> Result<Option<Capsule>> {
    let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;

    let affected = tx
        .execute(
            "UPDATE capsules
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
        .context("no se pudo actualizar la capsule")?;

    if affected == 0 {
        return Ok(None);
    }

    tx.execute(
        "INSERT INTO dispense_log (action, pill_id) VALUES ('capsule_revise', ?1)",
        params![id],
    )?;

    let capsule = tx
        .query_row(
            "SELECT id, sync_id, compound, title, content, created_at, updated_at
             FROM capsules WHERE id = ?1",
            params![id],
            row_to_capsule,
        )
        .context("no se pudo leer la capsule revisada")?;

    tx.commit()?;
    Ok(Some(capsule))
}

/// Soft delete de una capsule.
/// Devuelve `None` si no existe o ya estaba descartada.
pub fn discard(conn: &mut Connection, id: i64) -> Result<Option<CapsuleDiscardResult>> {
    let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;

    let affected = tx
        .execute(
            "UPDATE capsules SET deleted_at = datetime('now')
         WHERE id = ?1 AND deleted_at IS NULL",
            params![id],
        )
        .context("no se pudo descartar la capsule")?;

    if affected == 0 {
        return Ok(None);
    }

    let deleted_at: String = tx.query_row(
        "SELECT deleted_at FROM capsules WHERE id = ?1",
        params![id],
        |row| row.get(0),
    )?;

    tx.execute(
        "INSERT INTO dispense_log (action, pill_id) VALUES ('capsule_discard', ?1)",
        params![id],
    )?;

    tx.commit()?;
    Ok(Some(CapsuleDiscardResult { id, deleted_at }))
}

fn row_to_capsule(row: &rusqlite::Row<'_>) -> rusqlite::Result<Capsule> {
    Ok(Capsule {
        id: row.get(0)?,
        sync_id: row.get(1)?,
        compound: row.get(2)?,
        title: row.get(3)?,
        content: row.get(4)?,
        created_at: row.get(5)?,
        updated_at: row.get(6)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::connection::open_in_memory;
    use crate::domain::capsule::{CapsuleCompound, CapsulePatch, NewCapsule};

    fn sample_capsule() -> NewCapsule {
        NewCapsule {
            title: "Snake_case siempre".into(),
            content: "Prefiero snake_case en todos los proyectos, incluso en TypeScript.".into(),
            compound: CapsuleCompound::Convention,
        }
    }

    #[test]
    fn take_and_read() {
        let mut conn = open_in_memory().unwrap();
        let result = take(&mut conn, &sample_capsule()).unwrap();
        assert_eq!(result.action, "created");

        let cap = read(&conn, result.id).unwrap().unwrap();
        assert_eq!(cap.compound, "convention");
        assert_eq!(cap.title, "Snake_case siempre");
    }

    #[test]
    fn revise_partial_patch() {
        let mut conn = open_in_memory().unwrap();
        let cap = take(&mut conn, &sample_capsule()).unwrap();

        let updated = revise(
            &mut conn,
            cap.id,
            &CapsulePatch {
                title: Some("Convención de nombres".into()),
                content: None,
                compound: None,
            },
        )
        .unwrap()
        .unwrap();

        assert_eq!(updated.title, "Convención de nombres");
        assert!(updated.content.contains("snake_case"));
    }

    #[test]
    fn discard_soft_deletes() {
        let mut conn = open_in_memory().unwrap();
        let cap = take(&mut conn, &sample_capsule()).unwrap();
        let result = discard(&mut conn, cap.id).unwrap().unwrap();
        assert_eq!(result.id, cap.id);
        assert!(read(&conn, cap.id).unwrap().is_none());
    }

    #[test]
    fn discard_twice_returns_none() {
        let mut conn = open_in_memory().unwrap();
        let cap = take(&mut conn, &sample_capsule()).unwrap();
        discard(&mut conn, cap.id).unwrap();
        assert!(discard(&mut conn, cap.id).unwrap().is_none());
    }
}
