//! Operaciones sobre la tabla `registered_bottles` de la DB global.
//!
//! La tabla `registered_bottles` actúa como registro centralizado de todas
//! las DBs (locales y globales) conocidas por el usuario. Es la fuente de
//! verdad para `conn_for_bottle` y `bottle_list`.

use anyhow::{Context, Result};
use rusqlite::{params, Connection};

/// Registro de una DB de bottle en la tabla `registered_bottles` de la DB global.
#[derive(Debug)]
pub struct RegisteredBottle {
    pub id: i64,
    pub bottle_id: String,
    pub name: String,
    pub display_name: String,
    pub db_path: String,
    pub registered_at: String,
    pub last_seen_at: String,
}

/// Registra una DB local en la tabla `registered_bottles` de la DB global.
///
/// Idempotente: INSERT OR IGNORE por UNIQUE constraint en db_path.
///
/// Devuelve `Ok(true)` si se insertó una fila nueva, `Ok(false)` si ya existía
/// (UNIQUE constraint ignorada).
pub fn register(
    conn: &Connection,
    bottle_id: &str,
    name: &str,
    display_name: &str,
    db_path: &str,
) -> Result<bool> {
    let changes = conn
        .execute(
            "INSERT OR IGNORE INTO registered_bottles (bottle_id, name, display_name, db_path)
         VALUES (?1, ?2, ?3, ?4)",
            params![bottle_id, name, display_name, db_path],
        )
        .context("failed to register bottle in global registry")?;
    Ok(changes > 0)
}

/// Actualiza la ruta de DB de un registro existente.
///
/// Devuelve `Ok(true)` si se actualizó, `Ok(false)` si el `id` no existe.
pub fn update_db_path(conn: &Connection, id: i64, new_db_path: &str) -> Result<bool> {
    let count = conn
        .execute(
            "UPDATE registered_bottles SET db_path = ?1 WHERE id = ?2",
            params![new_db_path, id],
        )
        .context("failed to update registered bottle path")?;
    Ok(count > 0)
}

/// Elimina un registro de la tabla `registered_bottles` por su `id` interno.
///
/// Devuelve `Ok(true)` si se eliminó, `Ok(false)` si no existía.
pub fn unregister(conn: &Connection, id: i64) -> Result<bool> {
    let count = conn
        .execute("DELETE FROM registered_bottles WHERE id = ?1", params![id])
        .context("failed to unregister bottle from global registry")?;
    Ok(count > 0)
}

/// Busca un registro por el UUID del bottle (completo o prefijo ≥8 chars).
///
/// La columna lookup es `bottle_id` (no `id`), por lo que no se puede usar
/// [`crate::db::store::id_resolver::resolve_id`] directamente: replicamos su
/// semántica aquí (`InvalidId` si prefijo < 8 chars, `AmbiguousId` si match
/// múltiple, `Ok(None)` si no hay match).
pub fn find_by_bottle_id(conn: &Connection, bottle_id: &str) -> Result<Option<RegisteredBottle>> {
    use crate::db::store::id_resolver::{normalize_prefix, MIN_PREFIX_LEN};
    use crate::error::PillboxError;

    if bottle_id.len() < MIN_PREFIX_LEN {
        return Err(PillboxError::InvalidId {
            id: bottle_id.to_string(),
        }
        .into());
    }

    let normalized = normalize_prefix(bottle_id);
    let pattern = format!("{}%", normalized);
    let mut stmt = conn.prepare(
        "SELECT id, bottle_id, name, display_name, db_path, registered_at, last_seen_at
         FROM registered_bottles WHERE bottle_id = ?1 OR bottle_id LIKE ?2
         LIMIT 3",
    )?;
    let rows: Vec<RegisteredBottle> = stmt
        .query_map(params![normalized, pattern], row_to_registered)?
        .collect::<rusqlite::Result<Vec<_>>>()
        .context("failed to find registered bottle by bottle_id")?;

    match rows.len() {
        0 => Ok(None),
        1 => Ok(Some(rows.into_iter().next().unwrap())),
        _ => Err(PillboxError::AmbiguousId {
            id_prefix: bottle_id.to_string(),
            candidates: rows.iter().map(|r| r.bottle_id.clone()).collect(),
        }
        .into()),
    }
}

/// Lista todos los registros ordenados por fecha de registro (más recientes primero).
pub fn list(conn: &Connection) -> Result<Vec<RegisteredBottle>> {
    let mut stmt = conn.prepare(
        "SELECT id, bottle_id, name, display_name, db_path, registered_at, last_seen_at
         FROM registered_bottles ORDER BY registered_at DESC",
    )?;
    let rows = stmt
        .query_map([], row_to_registered)?
        .collect::<rusqlite::Result<Vec<_>>>()
        .context("failed to list registered bottles")?;
    Ok(rows)
}

/// Mapea una fila de SQLite al tipo [`RegisteredBottle`].
fn row_to_registered(row: &rusqlite::Row<'_>) -> rusqlite::Result<RegisteredBottle> {
    Ok(RegisteredBottle {
        id: row.get(0)?,
        bottle_id: row.get(1)?,
        name: row.get(2)?,
        display_name: row.get(3)?,
        db_path: row.get(4)?,
        registered_at: row.get(5)?,
        last_seen_at: row.get(6)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::connection::open_in_memory;
    use crate::error::PillboxError;

    const BOTTLE_UUID: &str = "019db1d0-bd9e-7940-aa29-054b250450ec";

    #[test]
    fn register_and_list() {
        let conn = open_in_memory().unwrap();
        register(
            &conn,
            BOTTLE_UUID,
            "mi-proyecto",
            "Mi Proyecto",
            "/home/user/.pillbox/pillbox.db",
        )
        .unwrap();

        let rows = list(&conn).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].name, "mi-proyecto");
        assert_eq!(rows[0].display_name, "Mi Proyecto");
        assert_eq!(rows[0].db_path, "/home/user/.pillbox/pillbox.db");
        assert_eq!(rows[0].bottle_id, BOTTLE_UUID);
        assert!(rows[0].id > 0);
    }

    #[test]
    fn register_idempotent() {
        let conn = open_in_memory().unwrap();
        register(&conn, BOTTLE_UUID, "proj", "Proj", "/tmp/proj.db").unwrap();
        register(&conn, BOTTLE_UUID, "proj", "Proj", "/tmp/proj.db").unwrap();

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
        let uuid_b = "019db1d0-bd9e-7940-aa29-054b250450ed";
        register(&conn, BOTTLE_UUID, "a", "A", "/tmp/a.db").unwrap();
        register(&conn, uuid_b, "b", "B", "/tmp/b.db").unwrap();

        let rows = list(&conn).unwrap();
        assert_eq!(rows.len(), 2);
    }

    #[test]
    fn find_by_bottle_id_ok() {
        let conn = open_in_memory().unwrap();
        register(&conn, BOTTLE_UUID, "proj", "Proj", "/tmp/proj.db").unwrap();
        let found = find_by_bottle_id(&conn, BOTTLE_UUID).unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().bottle_id, BOTTLE_UUID);
    }

    #[test]
    fn find_by_bottle_id_missing() {
        let conn = open_in_memory().unwrap();
        let found = find_by_bottle_id(&conn, "uuid-inexistente").unwrap();
        assert!(found.is_none());
    }

    #[test]
    fn find_by_bottle_id_8char_prefix() {
        let conn = open_in_memory().unwrap();
        register(&conn, BOTTLE_UUID, "proj", "Proj", "/tmp/proj.db").unwrap();
        // "019db1d0" son los primeros 8 chars del UUID
        let found = find_by_bottle_id(&conn, "019db1d0").unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().bottle_id, BOTTLE_UUID);
    }

    #[test]
    fn find_by_bottle_id_12char_prefix_without_dashes() {
        let conn = open_in_memory().unwrap();
        register(&conn, BOTTLE_UUID, "proj", "Proj", "/tmp/proj.db").unwrap();
        // "019db1d0bd9e" son los primeros 12 hex chars sin guiones
        let found = find_by_bottle_id(&conn, "019db1d0bd9e").unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().bottle_id, BOTTLE_UUID);
    }

    #[test]
    fn find_by_bottle_id_12char_prefix_with_dashes() {
        let conn = open_in_memory().unwrap();
        register(&conn, BOTTLE_UUID, "proj", "Proj", "/tmp/proj.db").unwrap();
        // Formato con guion: "019db1d0-bd9e"
        let found = find_by_bottle_id(&conn, "019db1d0-bd9e").unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().bottle_id, BOTTLE_UUID);
    }

    #[test]
    fn find_by_bottle_id_too_short_returns_invalid_id() {
        let conn = open_in_memory().unwrap();
        let err = find_by_bottle_id(&conn, "abc").unwrap_err();
        let pe = err.downcast_ref::<PillboxError>().expect("PillboxError");
        assert!(matches!(pe, PillboxError::InvalidId { .. }));
    }

    #[test]
    fn find_by_bottle_id_ambiguous_returns_ambiguous_id() {
        let conn = open_in_memory().unwrap();
        let uuid_a = "019db1d0-aaaa-7000-aa00-000000000000";
        let uuid_b = "019db1d0-bbbb-7000-bb00-000000000000";
        register(&conn, uuid_a, "a", "A", "/tmp/a.db").unwrap();
        register(&conn, uuid_b, "b", "B", "/tmp/b.db").unwrap();
        let err = find_by_bottle_id(&conn, "019db1d0").unwrap_err();
        let pe = err.downcast_ref::<PillboxError>().expect("PillboxError");
        assert!(matches!(pe, PillboxError::AmbiguousId { .. }));
    }
}
