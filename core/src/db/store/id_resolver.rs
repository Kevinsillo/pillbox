//! Resolución de IDs por prefijo corto.
//!
//! Permite a los callers usar prefijos de UUID (mínimo 8 caracteres) en lugar
//! del UUID completo (36 caracteres). El helper [`resolve_id`] devuelve el
//! UUID completo si el prefijo es inequívoco, o un error tipado si es ambiguo
//! o demasiado corto.

use anyhow::Result;
use rusqlite::{params, Connection};

use crate::error::PillboxError;

/// Longitud mínima de prefijo aceptada para evitar matches demasiado amplios.
pub const MIN_PREFIX_LEN: usize = 8;

/// Longitud de un UUID completo en formato canónico (`xxxxxxxx-xxxx-...`).
const FULL_UUID_LEN: usize = 36;

/// Número máximo de candidatos a recuperar al resolver un prefijo.
///
/// Se piden hasta 3 para poder devolver al menos 2 en el error de ambigüedad
/// (el tercero se usa como indicador de "hay más", aunque por ahora todos los
/// recuperados se devuelven en `candidates`).
const RESOLVE_LIMIT: usize = 3;

/// Resuelve un id (UUID completo o prefijo) en una tabla con columna `id TEXT`.
///
/// Devuelve:
/// - `Err(InvalidId)` si `id.len() < MIN_PREFIX_LEN`.
/// - `Ok(Some(uuid))` si encuentra exactamente un match (o si `id` ya es un
///   UUID completo de 36 chars, en cuyo caso lo devuelve sin consultar la DB).
/// - `Ok(None)` si no hay matches — el caller decide si traducirlo a
///   `XxxNotFound` o similar.
/// - `Err(AmbiguousId)` si hay 2 o más matches.
///
/// **Nota de seguridad**: `table` se interpola en la query con `format!` porque
/// es un literal interno (no input de usuario). El `id` se pasa como parámetro
/// vinculado para evitar inyección.
pub fn resolve_id(conn: &Connection, table: &str, id: &str) -> Result<Option<String>> {
    if id.len() < MIN_PREFIX_LEN {
        return Err(PillboxError::InvalidId { id: id.to_string() }.into());
    }

    // Shortcut: UUID completo no puede ser ambiguo — evitar consulta innecesaria.
    if id.len() == FULL_UUID_LEN {
        return Ok(Some(id.to_string()));
    }

    let sql = format!("SELECT id FROM {} WHERE id LIKE ?1 LIMIT {}", table, RESOLVE_LIMIT);
    let pattern = format!("{}%", id);

    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt
        .query_map(params![pattern], |row| row.get::<_, String>(0))?
        .collect::<rusqlite::Result<Vec<String>>>()?;

    match rows.len() {
        0 => Ok(None),
        1 => Ok(Some(rows.into_iter().next().unwrap())),
        _ => Err(PillboxError::AmbiguousId {
            id_prefix: id.to_string(),
            candidates: rows,
        }
        .into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup_conn() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute(
            "CREATE TABLE test_items (id TEXT PRIMARY KEY)",
            [],
        )
        .unwrap();
        conn
    }

    fn insert(conn: &Connection, id: &str) {
        conn.execute("INSERT INTO test_items (id) VALUES (?1)", params![id])
            .unwrap();
    }

    #[test]
    fn returns_invalid_id_when_prefix_too_short() {
        let conn = setup_conn();
        let err = resolve_id(&conn, "test_items", "abc").unwrap_err();
        let pe = err.downcast_ref::<PillboxError>().expect("PillboxError");
        match pe {
            PillboxError::InvalidId { id } => assert_eq!(id, "abc"),
            other => panic!("expected InvalidId, got {:?}", other),
        }
    }

    #[test]
    fn returns_none_when_no_match() {
        let conn = setup_conn();
        let result = resolve_id(&conn, "test_items", "deadbeef").unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn returns_full_uuid_for_unique_prefix() {
        let conn = setup_conn();
        let uuid = "019e1d3e-f211-77d3-9e62-0c421ae1b938";
        insert(&conn, uuid);

        let result = resolve_id(&conn, "test_items", "019e1d3e").unwrap();
        assert_eq!(result, Some(uuid.to_string()));
    }

    #[test]
    fn returns_ambiguous_when_multiple_matches() {
        let conn = setup_conn();
        let uuid_a = "01234567-aaaa-bbbb-cccc-dddddddddddd";
        let uuid_b = "01234567-eeee-ffff-1111-222222222222";
        insert(&conn, uuid_a);
        insert(&conn, uuid_b);

        let err = resolve_id(&conn, "test_items", "01234567").unwrap_err();
        let pe = err.downcast_ref::<PillboxError>().expect("PillboxError");
        match pe {
            PillboxError::AmbiguousId { id_prefix, candidates } => {
                assert_eq!(id_prefix, "01234567");
                assert_eq!(candidates.len(), 2);
                assert!(candidates.contains(&uuid_a.to_string()));
                assert!(candidates.contains(&uuid_b.to_string()));
            }
            other => panic!("expected AmbiguousId, got {:?}", other),
        }
    }

    #[test]
    fn full_uuid_short_circuits_without_db_query() {
        // El UUID completo se devuelve tal cual sin consultar la DB —
        // verificamos pasando una tabla inexistente: si hubiese query, fallaría.
        let conn = Connection::open_in_memory().unwrap();
        let uuid = "019e1d3e-f211-77d3-9e62-0c421ae1b938";
        let result = resolve_id(&conn, "nonexistent_table", uuid).unwrap();
        assert_eq!(result, Some(uuid.to_string()));
    }
}
