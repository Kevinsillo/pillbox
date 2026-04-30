use anyhow::{Context, Result};
use rusqlite::{params, Connection};
use uuid::Uuid;

use crate::{
    domain::bottle::{Bottle, NewBottle},
    error::PillboxError,
};

/// Crea un bottle nuevo en la DB.
pub fn create(conn: &mut Connection, input: &NewBottle) -> Result<Bottle> {
    let id = Uuid::now_v7().to_string();
    let tx = conn.transaction()?;

    tx.execute(
        "INSERT INTO bottles (id, name, display_name, directory, scope)
         VALUES (?1, ?2, ?3, ?4, ?5)",
        params![
            id,
            input.name,
            input.display_name,
            input.directory,
            input.scope.as_str()
        ],
    )
    .map_err(|e| {
        if let rusqlite::Error::SqliteFailure(ref f, _) = e {
            if f.code == rusqlite::ErrorCode::ConstraintViolation {
                return anyhow::anyhow!(PillboxError::BottleAlreadyExists {
                    name: input.name.clone(),
                });
            }
        }
        anyhow::anyhow!(e).context("failed to create bottle")
    })?;

    let bottle = tx
        .query_row(
            "SELECT id, name, display_name, directory, scope, created_at, last_seen_at
         FROM bottles WHERE id = ?1",
            params![id],
            row_to_bottle,
        )
        .context("failed to read newly created bottle")?;

    tx.commit()?;
    Ok(bottle)
}

/// Lista todos los bottles ordenados por fecha de creación.
pub fn list(conn: &Connection) -> Result<Vec<Bottle>> {
    let mut stmt = conn.prepare(
        "SELECT id, name, display_name, directory, scope, created_at, last_seen_at
         FROM bottles ORDER BY created_at DESC",
    )?;

    let bottles = stmt
        .query_map([], row_to_bottle)?
        .collect::<rusqlite::Result<Vec<_>>>()
        .context("failed to list bottles")?;

    Ok(bottles)
}

/// Busca un bottle por su ID (UUID).
pub fn find_by_id(conn: &Connection, id: &str) -> Result<Option<Bottle>> {
    match conn.query_row(
        "SELECT id, name, display_name, directory, scope, created_at, last_seen_at
         FROM bottles WHERE id = ?1",
        params![id],
        row_to_bottle,
    ) {
        Ok(b) => Ok(Some(b)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(e).context("failed to find bottle by id"),
    }
}

/// Busca un bottle por el directorio del proyecto.
pub fn find_by_directory(conn: &Connection, directory: &str) -> Result<Option<Bottle>> {
    match conn.query_row(
        "SELECT id, name, display_name, directory, scope, created_at, last_seen_at
         FROM bottles WHERE directory = ?1",
        params![directory],
        row_to_bottle,
    ) {
        Ok(b) => Ok(Some(b)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(e).context("failed to find bottle by directory"),
    }
}

/// Elimina un bottle y en cascada sus prescriptions y pills.
pub fn delete(conn: &mut Connection, id: &str) -> Result<bool> {
    let tx = conn.transaction()?;

    // dispense_log referencia prescriptions(id) y pills(id) — borrar antes que ambas.
    tx.execute(
        "DELETE FROM dispense_log WHERE prescription_id IN (SELECT id FROM prescriptions WHERE bottle_id = ?1)",
        params![id],
    )
    .context("failed to delete bottle dispense_log")?;

    // pill_links referencia pills(id) — borrar antes que pills.
    tx.execute(
        "DELETE FROM pill_links WHERE from_id IN (SELECT id FROM pills WHERE prescription_id IN (SELECT id FROM prescriptions WHERE bottle_id = ?1)) OR to_id IN (SELECT id FROM pills WHERE prescription_id IN (SELECT id FROM prescriptions WHERE bottle_id = ?1))",
        params![id],
    )
    .context("failed to delete bottle pill_links")?;

    tx.execute(
        "DELETE FROM pills WHERE prescription_id IN (SELECT id FROM prescriptions WHERE bottle_id = ?1)",
        params![id],
    )
    .context("failed to delete bottle pills")?;

    tx.execute(
        "DELETE FROM prescriptions WHERE bottle_id = ?1",
        params![id],
    )
    .context("failed to delete bottle prescriptions")?;

    let count = tx
        .execute("DELETE FROM bottles WHERE id = ?1", params![id])
        .context("failed to delete bottle")?;

    tx.commit()?;
    Ok(count > 0)
}

fn row_to_bottle(row: &rusqlite::Row<'_>) -> rusqlite::Result<Bottle> {
    Ok(Bottle {
        id: row.get(0)?,
        name: row.get(1)?,
        display_name: row.get(2)?,
        directory: row.get(3)?,
        scope: row.get(4)?,
        created_at: row.get(5)?,
        last_seen_at: row.get(6)?,
        linked: true,
        reg_id: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::connection::open_in_memory;
    use crate::domain::bottle::{BottleScope, NewBottle};

    fn test_bottle(name: &str, dir: &str) -> NewBottle {
        NewBottle {
            name: name.into(),
            display_name: name.to_uppercase(),
            directory: dir.into(),
            scope: BottleScope::Local,
        }
    }

    #[test]
    fn create_and_find() {
        let mut conn = open_in_memory().unwrap();
        let b = create(&mut conn, &test_bottle("mi-proyecto", "/tmp/mi-proyecto")).unwrap();
        assert_eq!(b.name, "mi-proyecto");
        assert_eq!(b.scope, "local");

        let found = find_by_id(&conn, &b.id).unwrap().unwrap();
        assert_eq!(found.id, b.id);
    }

    #[test]
    fn uuid_is_generated_on_create() {
        let mut conn = open_in_memory().unwrap();
        let b = create(&mut conn, &test_bottle("uuid-test", "/tmp/uuid-test")).unwrap();
        assert!(!b.id.is_empty());
        assert_eq!(b.id.len(), 36);
        assert!(b.id.contains('-'));
    }

    #[test]
    fn two_bottles_have_different_ids() {
        let mut conn = open_in_memory().unwrap();
        let a = create(&mut conn, &test_bottle("a", "/tmp/a")).unwrap();
        let b = create(&mut conn, &test_bottle("b", "/tmp/b")).unwrap();
        assert_ne!(a.id, b.id);
    }

    #[test]
    fn find_by_directory_ok() {
        let mut conn = open_in_memory().unwrap();
        create(&mut conn, &test_bottle("proj", "/home/kevin/proj")).unwrap();
        let found = find_by_directory(&conn, "/home/kevin/proj").unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().name, "proj");
    }

    #[test]
    fn list_returns_all() {
        let mut conn = open_in_memory().unwrap();
        create(&mut conn, &test_bottle("a", "/tmp/a")).unwrap();
        create(&mut conn, &test_bottle("b", "/tmp/b")).unwrap();
        assert_eq!(list(&conn).unwrap().len(), 2);
    }

    #[test]
    fn duplicate_name_fails() {
        let mut conn = open_in_memory().unwrap();
        create(&mut conn, &test_bottle("dup", "/tmp/dup1")).unwrap();
        let err = create(&mut conn, &test_bottle("dup", "/tmp/dup2")).unwrap_err();
        let typed = err.downcast::<PillboxError>().unwrap();
        assert!(matches!(typed, PillboxError::BottleAlreadyExists { .. }));
    }

    #[test]
    fn find_by_id_missing_returns_none() {
        let conn = open_in_memory().unwrap();
        assert!(find_by_id(&conn, "id-inexistente").unwrap().is_none());
    }

    #[test]
    fn find_by_directory_missing_returns_none() {
        let conn = open_in_memory().unwrap();
        assert!(find_by_directory(&conn, "/ruta/inexistente")
            .unwrap()
            .is_none());
    }

    #[test]
    fn list_empty_returns_empty_vec() {
        let conn = open_in_memory().unwrap();
        assert!(list(&conn).unwrap().is_empty());
    }

}
