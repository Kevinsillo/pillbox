//! Handler MCP para listar los compounds disponibles de pills y capsules.

use serde::Serialize;
use serde_json::Value;

use crate::mcp::response::{anyhow_to_response, Conn, Response};

/// Lista los compounds activos de la tabla `pill_compounds` o `capsule_compounds`.
///
/// El nombre de la tabla se infiere del nombre de la herramienta (`tool`).
pub fn list(conn: &mut Conn, tool: &str, _input: Value) -> Response {
    #[derive(Serialize)]
    struct CompoundEntry {
        id: String,
        description: String,
        prompt_hint: String,
    }
    let table = if tool == "pill_compounds" {
        "pill_compounds"
    } else {
        "capsule_compounds"
    };
    let sql = format!(
        "SELECT id, description, prompt_hint FROM {} WHERE is_active = 1",
        table
    );
    match conn.prepare(&sql).and_then(|mut s| {
        s.query_map([], |row| {
            Ok(CompoundEntry {
                id: row.get(0)?,
                description: row.get(1)?,
                prompt_hint: row.get(2)?,
            })
        })
        .and_then(|rows| rows.collect::<rusqlite::Result<Vec<_>>>())
    }) {
        Ok(entries) => Response::ok(entries),
        Err(e) => anyhow_to_response(anyhow::anyhow!("{}", e)),
    }
}
