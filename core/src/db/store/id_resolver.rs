//! Resolución e identificación corta de UUIDs.
//!
//! ## Formato de ID corto (display)
//!
//! UUID v7 codifica el timestamp en milisegundos en los primeros 48 bits.
//! Mostrar solo 8 hex chars (32 bits) causaba colisiones para registros creados
//! en la misma ventana de ~65 segundos. El formato canónico usa **12 hex chars
//! sin guiones**, que cubren el timestamp completo:
//!
//! ```text
//! "019e1d52-1a89-7d41-b9c0-1cc0f730b512"  →  "019e1d521a89"
//! ```
//!
//! Con 12 chars dos registros solo colisionarían si se crean en el mismo
//! milisegundo exacto, lo que es prácticamente imposible en uso normal.
//!
//! ## Resolución por prefijo
//!
//! [`resolve_id`] acepta prefijos **con o sin guiones** (mínimo [`MIN_PREFIX_LEN`]
//! chars), normalizando internamente antes del LIKE para que ambos formatos
//! funcionen contra la columna `id` (que almacena UUIDs con guiones).

use anyhow::Result;
use rusqlite::{params, Connection};

use crate::error::PillboxError;

/// Longitud mínima de prefijo aceptada para evitar matches demasiado amplios.
pub const MIN_PREFIX_LEN: usize = 8;

/// Longitud canónica del ID corto para display (12 hex chars sin guiones).
/// Cubre los 48 bits del timestamp UUID v7 → virtualmente libre de colisiones.
pub const DISPLAY_ID_LEN: usize = 12;

/// Longitud de un UUID completo con guiones (`xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx`).
const FULL_UUID_LEN: usize = 36;

/// Longitud de un UUID completo sin guiones (32 hex chars).
const FULL_UUID_NO_DASH_LEN: usize = 32;

/// Número máximo de candidatos a recuperar al resolver un prefijo.
///
/// Se piden hasta 3 para poder devolver al menos 2 en el error de ambigüedad.
const RESOLVE_LIMIT: usize = 3;

/// Devuelve los primeros [`DISPLAY_ID_LEN`] hex chars del UUID, sin guiones.
///
/// Usado en todas las salidas (CLI, MCP, WebUI) para mostrar IDs legibles y
/// prácticamente únicos. Ejemplo: `"019e1d52-1a89-..."` → `"019e1d521a89"`.
pub fn display_id(uuid: &str) -> String {
    uuid.chars()
        .filter(|c| *c != '-')
        .take(DISPLAY_ID_LEN)
        .collect()
}

/// Convierte un prefijo sin guiones al formato UUID con guiones, necesario para
/// construir el patrón LIKE contra la columna `id` (que almacena UUIDs con guiones).
///
/// Los guiones se insertan en las posiciones estándar del UUID tras los hex chars
/// 8, 12, 16 y 20. Si el input ya contiene guiones se devuelve sin cambio.
///
/// Ejemplos:
/// - `"019e1d521a89"` → `"019e1d52-1a89"` (prefijo de display, 12 chars)
/// - `"019e1d52"` → `"019e1d52"` (sin cambio, guion no alcanzado)
/// - `"019e1d52-1a89"` → `"019e1d52-1a89"` (ya tiene guiones)
fn normalize_prefix(id: &str) -> String {
    if id.contains('-') {
        return id.to_string();
    }
    // Reinserta guiones en las posiciones estándar UUID v7:
    // XXXXXXXX-XXXX-XXXX-XXXX-XXXXXXXXXXXX  (índices en la cadena sin guiones)
    // ^0      ^8   ^12  ^16  ^20
    let mut out = String::with_capacity(id.len() + 4);
    for (i, c) in id.chars().enumerate() {
        if i == 8 || i == 12 || i == 16 || i == 20 {
            out.push('-');
        }
        out.push(c);
    }
    out
}

/// Resuelve un id (UUID completo o prefijo) en una tabla con columna `id TEXT`.
///
/// Acepta prefijos **con o sin guiones** — internamente normaliza el input antes
/// de construir el patrón LIKE, por lo que `"019e1d521a89"` y `"019e1d52-1a89"`
/// son equivalentes.
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
/// vinculado para evitar inyección SQL.
pub fn resolve_id(conn: &Connection, table: &str, id: &str) -> Result<Option<String>> {
    if id.len() < MIN_PREFIX_LEN {
        return Err(PillboxError::InvalidId { id: id.to_string() }.into());
    }

    // Short-circuit: UUID completo (con o sin guiones) — no puede ser ambiguo.
    // Se normaliza por si llega sin guiones. No verifica existencia; el caller
    // debe manejar el caso Not Found si es necesario.
    if id.len() == FULL_UUID_LEN
        || (id.len() == FULL_UUID_NO_DASH_LEN && !id.contains('-'))
    {
        return Ok(Some(normalize_prefix(id)));
    }

    // Normalizar el prefijo antes de construir el patrón LIKE. Un input sin
    // guiones como "019e1d521a89" se convierte a "019e1d52-1a89" para que el
    // LIKE coincida con los UUIDs almacenados en formato canónico con guiones.
    let normalized = normalize_prefix(id);
    let pattern = format!("{}%", normalized);

    let sql = format!(
        "SELECT id FROM {} WHERE id LIKE ?1 LIMIT {}",
        table, RESOLVE_LIMIT
    );
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
        conn.execute("CREATE TABLE test_items (id TEXT PRIMARY KEY)", [])
            .unwrap();
        conn
    }

    fn insert(conn: &Connection, id: &str) {
        conn.execute("INSERT INTO test_items (id) VALUES (?1)", params![id])
            .unwrap();
    }

    #[test]
    fn display_id_strips_dashes_and_takes_12_chars() {
        let uuid = "019e1d52-1a89-7d41-b9c0-1cc0f730b512";
        assert_eq!(display_id(uuid), "019e1d521a89");
    }

    #[test]
    fn normalize_prefix_inserts_dashes_at_correct_positions() {
        assert_eq!(normalize_prefix("019e1d521a89"), "019e1d52-1a89");
        assert_eq!(normalize_prefix("019e1d52"), "019e1d52");
        assert_eq!(normalize_prefix("019e1d52-1a89"), "019e1d52-1a89");
        assert_eq!(
            normalize_prefix("019e1d521a897d41"),
            "019e1d52-1a89-7d41"
        );
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
    fn returns_full_uuid_for_unique_prefix_with_dashes() {
        let conn = setup_conn();
        let uuid = "019e1d3e-f211-77d3-9e62-0c421ae1b938";
        insert(&conn, uuid);
        let result = resolve_id(&conn, "test_items", "019e1d3e").unwrap();
        assert_eq!(result, Some(uuid.to_string()));
    }

    #[test]
    fn returns_full_uuid_for_nodash_display_prefix() {
        // El formato de display "019e1d3ef211" (sin guiones) debe resolverse igual
        // que "019e1d3e-f211" (con guiones).
        let conn = setup_conn();
        let uuid = "019e1d3e-f211-77d3-9e62-0c421ae1b938";
        insert(&conn, uuid);
        let result = resolve_id(&conn, "test_items", "019e1d3ef211").unwrap();
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
            PillboxError::AmbiguousId {
                id_prefix,
                candidates,
            } => {
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
        // UUID completo se devuelve sin consultar la DB.
        let conn = Connection::open_in_memory().unwrap();
        let uuid = "019e1d3e-f211-77d3-9e62-0c421ae1b938";
        let result = resolve_id(&conn, "nonexistent_table", uuid).unwrap();
        assert_eq!(result, Some(uuid.to_string()));
    }

    #[test]
    fn full_uuid_no_dash_short_circuits() {
        // UUID completo sin guiones también hace short-circuit y se normaliza.
        let conn = Connection::open_in_memory().unwrap();
        let uuid_no_dash = "019e1d3ef21177d39e620c421ae1b938";
        let uuid_with_dash = "019e1d3e-f211-77d3-9e62-0c421ae1b938";
        let result = resolve_id(&conn, "nonexistent_table", uuid_no_dash).unwrap();
        assert_eq!(result, Some(uuid_with_dash.to_string()));
    }
}
