use anyhow::{Context, Result};
use rusqlite::{params, Connection};
use uuid::Uuid;

pub struct RegisteredBottle {
    pub id: String,
    pub name: String,
    pub display_name: String,
    pub db_path: String,
    pub registered_at: String,
    pub last_seen_at: String,
}

/// Registra una DB local en la tabla `registered_bottles` de la DB global.
///
/// Idempotente: INSERT OR IGNORE por UNIQUE constraint en db_path.
pub fn register(conn: &Connection, name: &str, display_name: &str, db_path: &str) -> Result<()> {
    let id = Uuid::now_v7().to_string();
    conn.execute(
        "INSERT OR IGNORE INTO registered_bottles (id, name, display_name, db_path)
         VALUES (?1, ?2, ?3, ?4)",
        params![id, name, display_name, db_path],
    )
    .context("no se pudo registrar el bottle en el registry global")?;
    Ok(())
}

pub fn list(conn: &Connection) -> Result<Vec<RegisteredBottle>> {
    let mut stmt = conn.prepare(
        "SELECT id, name, display_name, db_path, registered_at, last_seen_at
         FROM registered_bottles ORDER BY registered_at DESC",
    )?;
    let rows = stmt
        .query_map([], |row| {
            Ok(RegisteredBottle {
                id: row.get(0)?,
                name: row.get(1)?,
                display_name: row.get(2)?,
                db_path: row.get(3)?,
                registered_at: row.get(4)?,
                last_seen_at: row.get(5)?,
            })
        })?
        .collect::<rusqlite::Result<Vec<_>>>()
        .context("no se pudo listar los bottles registrados")?;
    Ok(rows)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::connection::open_in_memory;

    #[test]
    fn register_and_list() {
        let conn = open_in_memory().unwrap();
        register(&conn, "mi-proyecto", "Mi Proyecto", "/home/user/.pillbox/pillbox.db").unwrap();

        let rows = list(&conn).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].name, "mi-proyecto");
        assert_eq!(rows[0].display_name, "Mi Proyecto");
        assert_eq!(rows[0].db_path, "/home/user/.pillbox/pillbox.db");
        assert!(!rows[0].id.is_empty());
    }

    #[test]
    fn register_idempotent() {
        let conn = open_in_memory().unwrap();
        register(&conn, "proj", "Proj", "/tmp/proj.db").unwrap();
        register(&conn, "proj", "Proj", "/tmp/proj.db").unwrap(); // INSERT OR IGNORE

        let rows = list(&conn).unwrap();
        assert_eq!(rows.len(), 1);
    }

    #[test]
    fn list_empty_returns_empty_vec() {
        let conn = open_in_memory().unwrap();
        assert!(list(&conn).unwrap().is_empty());
    }

    #[test]
    fn register_multiple_different_paths() {
        let conn = open_in_memory().unwrap();
        register(&conn, "a", "A", "/tmp/a.db").unwrap();
        register(&conn, "b", "B", "/tmp/b.db").unwrap();

        let rows = list(&conn).unwrap();
        assert_eq!(rows.len(), 2);
    }
}
