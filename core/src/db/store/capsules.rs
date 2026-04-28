use anyhow::{Context, Result};
use rusqlite::{params, Connection, TransactionBehavior};
use serde::Serialize;
use uuid::Uuid;

use crate::domain::capsule::{Capsule, CapsulePatch, NewCapsule};

// ─── Tipos de resultado ───────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct CapsuleStoreResult {
    pub id: i64,
    pub sync_id: String,
    pub action: &'static str, // "created"
    pub title: String,
    pub compound: String,
    pub content: String,
}

#[derive(Debug, Serialize)]
pub struct CapsuleDiscardResult {
    pub id: i64,
    pub deleted_at: String,
}

// ─── Store ────────────────────────────────────────────────────────────────────

/// Guarda una capsule de conocimiento personal (sin prescription ni bottle).
pub fn take(conn: &mut Connection, input: &NewCapsule) -> Result<CapsuleStoreResult> {
    let sync_id = Uuid::now_v7().to_string();
    let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;

    tx.execute(
        "INSERT INTO capsules (sync_id, compound, title, content)
         VALUES (?1, ?2, ?3, ?4)",
        params![sync_id, input.compound.as_str(), input.title, input.content],
    )
    .context("failed to insert capsule")?;

    let id = tx.last_insert_rowid();

    tx.execute(
        "INSERT INTO dispense_log (action, pill_id) VALUES ('capsule_store', ?1)",
        params![id],
    )?;

    tx.commit()?;
    Ok(CapsuleStoreResult {
        id,
        sync_id,
        action: "created",
        title: input.title.clone(),
        compound: input.compound.as_str().to_string(),
        content: input.content.clone(),
    })
}

/// Lee una capsule completa por ID (no descartada).
pub fn read(conn: &Connection, id: i64) -> Result<Option<Capsule>> {
    match conn.query_row(
        "SELECT id, sync_id, compound, title, content, created_at, updated_at, deleted_at
         FROM capsules WHERE id = ?1 AND deleted_at IS NULL",
        params![id],
        row_to_capsule,
    ) {
        Ok(c) => Ok(Some(c)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(e).context("failed to read capsule"),
    }
}

/// Lee una capsule completa por ID incluyendo descartadas.
pub fn read_any(conn: &Connection, id: i64) -> Result<Option<Capsule>> {
    match conn.query_row(
        "SELECT id, sync_id, compound, title, content, created_at, updated_at, deleted_at
         FROM capsules WHERE id = ?1",
        params![id],
        row_to_capsule,
    ) {
        Ok(c) => Ok(Some(c)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(e).context("failed to read capsule"),
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
        .context("failed to update capsule")?;

    if affected == 0 {
        return Ok(None);
    }

    tx.execute(
        "INSERT INTO dispense_log (action, pill_id) VALUES ('capsule_revise', ?1)",
        params![id],
    )?;

    let capsule = tx
        .query_row(
            "SELECT id, sync_id, compound, title, content, created_at, updated_at, deleted_at
             FROM capsules WHERE id = ?1",
            params![id],
            row_to_capsule,
        )
        .context("failed to read revised capsule")?;

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
        .context("failed to discard capsule")?;

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

/// Lista capsules globales con filtros opcionales (incluye archivadas).
pub fn list(conn: &Connection, limit: Option<u32>, compound: Option<&str>) -> Result<Vec<Capsule>> {
    let limit = limit.unwrap_or(50);
    let mut stmt = conn.prepare(
        "SELECT id, sync_id, compound, title, content, created_at, updated_at, deleted_at
         FROM capsules
         WHERE (?1 IS NULL OR compound = ?1)
         ORDER BY updated_at DESC
         LIMIT ?2",
    )?;
    let capsules = stmt
        .query_map(params![compound, limit], row_to_capsule)?
        .collect::<rusqlite::Result<Vec<_>>>()
        .context("failed to list capsules")?;
    Ok(capsules)
}

/// Hard delete de una capsule (irreversible).
///
/// Elimina en orden: dispense_log WHERE pill_id=? → capsules WHERE id=?.
/// Devuelve `Ok(true)` si existía y fue eliminada, `Ok(false)` si no existía.
pub fn hard_delete(conn: &mut Connection, id: i64) -> Result<bool> {
    let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;

    let exists: bool = tx.query_row(
        "SELECT EXISTS(SELECT 1 FROM capsules WHERE id = ?1)",
        params![id],
        |row| row.get(0),
    )?;

    if !exists {
        return Ok(false);
    }

    // 1. Eliminar entradas de dispense_log referenciando esta capsule
    tx.execute(
        "DELETE FROM dispense_log WHERE pill_id = ?1",
        params![id],
    )
    .context("failed to delete dispense_log for capsule")?;

    // 2. Eliminar la capsule
    tx.execute("DELETE FROM capsules WHERE id = ?1", params![id])
        .context("failed to delete capsule")?;

    tx.commit()?;
    Ok(true)
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
        deleted_at: row.get(7)?,
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
        let input = sample_capsule();
        let result = take(&mut conn, &input).unwrap();
        assert_eq!(result.action, "created");
        assert_eq!(result.title, input.title);
        assert_eq!(result.compound, input.compound.as_str());
        assert_eq!(result.content, input.content);

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
            &CapsulePatch {
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
        let cap = take(&mut conn, &sample_capsule()).unwrap();
        discard(&mut conn, cap.id).unwrap();

        let result = revise(
            &mut conn,
            cap.id,
            &CapsulePatch {
                title: Some("nuevo".into()),
                content: None,
                compound: None,
            },
        )
        .unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn list_returns_all_active() {
        let mut conn = open_in_memory().unwrap();
        take(&mut conn, &sample_capsule()).unwrap();
        take(
            &mut conn,
            &NewCapsule {
                title: "Otra convención".into(),
                content: "Usar PascalCase para tipos.".into(),
                compound: CapsuleCompound::Convention,
            },
        )
        .unwrap();

        let all = list(&conn, None, None).unwrap();
        assert_eq!(all.len(), 2);
    }

    #[test]
    fn list_filters_by_compound() {
        let mut conn = open_in_memory().unwrap();
        take(&mut conn, &sample_capsule()).unwrap(); // convention
        take(
            &mut conn,
            &NewCapsule {
                title: "Workflow de deploy".into(),
                content: "push a main = deploy automático.".into(),
                compound: CapsuleCompound::Workflow,
            },
        )
        .unwrap();

        let conventions = list(&conn, None, Some("convention")).unwrap();
        assert_eq!(conventions.len(), 1);
        assert_eq!(conventions[0].compound, "convention");
    }

    #[test]
    fn list_respects_limit() {
        let mut conn = open_in_memory().unwrap();
        for i in 0..5 {
            take(
                &mut conn,
                &NewCapsule {
                    title: format!("Cap {i}"),
                    content: "contenido".into(),
                    compound: CapsuleCompound::Manual,
                },
            )
            .unwrap();
        }

        let limited = list(&conn, Some(3), None).unwrap();
        assert_eq!(limited.len(), 3);
    }

    #[test]
    fn list_includes_discarded_after_filter_removal() {
        let mut conn = open_in_memory().unwrap();
        let cap = take(&mut conn, &sample_capsule()).unwrap();
        discard(&mut conn, cap.id).unwrap();

        let all = list(&conn, None, None).unwrap();
        assert_eq!(all.len(), 1);
        assert!(all[0].deleted_at.is_some());
    }

    #[test]
    fn read_any_finds_archived() {
        let mut conn = open_in_memory().unwrap();
        let cap = take(&mut conn, &sample_capsule()).unwrap();
        discard(&mut conn, cap.id).unwrap();

        let found = read_any(&conn, cap.id).unwrap();
        assert!(found.is_some());
        assert!(found.unwrap().deleted_at.is_some());
    }

    #[test]
    fn read_excludes_archived_but_read_any_does_not() {
        let mut conn = open_in_memory().unwrap();
        let cap = take(&mut conn, &sample_capsule()).unwrap();
        discard(&mut conn, cap.id).unwrap();

        assert!(read(&conn, cap.id).unwrap().is_none());
        assert!(read_any(&conn, cap.id).unwrap().is_some());
    }

    #[test]
    fn hard_delete_removes_row() {
        let mut conn = open_in_memory().unwrap();
        let cap = take(&mut conn, &sample_capsule()).unwrap();
        let deleted = hard_delete(&mut conn, cap.id).unwrap();
        assert!(deleted);
        assert!(read_any(&conn, cap.id).unwrap().is_none());
    }

    #[test]
    fn hard_delete_cleans_dispense_log() {
        let mut conn = open_in_memory().unwrap();
        let cap = take(&mut conn, &sample_capsule()).unwrap();
        hard_delete(&mut conn, cap.id).unwrap();

        let log_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM dispense_log WHERE pill_id = ?1",
                rusqlite::params![cap.id],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(log_count, 0);
    }

    #[test]
    fn hard_delete_nonexistent_returns_false() {
        let mut conn = open_in_memory().unwrap();
        let deleted = hard_delete(&mut conn, 9999).unwrap();
        assert!(!deleted);
    }

    #[test]
    fn list_includes_archived() {
        let mut conn = open_in_memory().unwrap();
        let cap = take(&mut conn, &sample_capsule()).unwrap();
        discard(&mut conn, cap.id).unwrap();

        let all = list(&conn, None, None).unwrap();
        assert_eq!(all.len(), 1);
        assert!(all[0].deleted_at.is_some());
    }
}
