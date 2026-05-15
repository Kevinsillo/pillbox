//! Enrutador de herramientas MCP — mapea el nombre de la herramienta al handler.

use rusqlite::Connection;
use serde_json::Value;

use super::{handlers, response::Response};

/// Despacha la herramienta MCP al handler correspondiente.
///
/// Devuelve [`Response::err`] con código `"unknown_tool"` si la herramienta
/// no está registrada.
pub fn dispatch(conn: &mut Connection, tool: &str, input: Value) -> Response {
    match tool {
        "pill_store" => handlers::pills::take(conn, input),
        "pill_read" => handlers::pills::read(conn, input),
        "pill_revise" => handlers::pills::revise(conn, input),
        "pill_discard" => handlers::pills::discard(conn, input),
        "pill_search" => handlers::pills::search(conn, input),
        "pill_compounds" => handlers::pills::compounds(conn, input),
        "bottle_context" => handlers::pills::bottle_context(conn, input),
        "prescription_context" => handlers::pills::prescription_context(conn, input),
        "capsule_store" => handlers::capsules::take(conn, input),
        "capsule_read" => handlers::capsules::read(conn, input),
        "capsule_revise" => handlers::capsules::revise(conn, input),
        "capsule_discard" => handlers::capsules::discard(conn, input),
        "capsule_search" => handlers::capsules::search(conn, input),
        "capsule_compounds" => handlers::capsules::compounds(conn, input),
        "prescription_open" => handlers::prescriptions::open(conn, input),
        "prescription_close" => handlers::prescriptions::close(conn, input),
        "prescription_reopen" => handlers::prescriptions::reopen(conn, input),
        "prescription_read" => handlers::prescriptions::read(conn, input),
        "prescription_discard" => handlers::prescriptions::discard(conn, input),
        "bottle_create" => handlers::bottles::create(conn, input),
        "bottle_list" => handlers::bottles::list(conn, input),
        "bottle_vinculate" => handlers::bottles::vinculate(conn, input),
        other => Response::err(
            "unknown_tool",
            format!("herramienta desconocida: '{}'", other),
        ),
    }
}
