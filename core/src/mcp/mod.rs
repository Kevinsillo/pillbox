//! Protocolo MCP (Model Context Protocol) de Pillbox.
//!
//! Lee JSON de stdin (`{"tool": "...", "input": {...}}`), ejecuta la operación
//! y escribe la respuesta JSON en stdout. Usado por `pillbox exec`.

use std::io::Read;

use anyhow::Result;
use serde::Deserialize;
use serde_json::Value;

use pillbox::{config, db};

mod dispatch;
mod handlers;
pub mod response;

use response::Response;

// ─── Protocolo ────────────────────────────────────────────────────────────────

/// Petición MCP deserializada desde stdin.
#[derive(Deserialize)]
struct Request {
    tool: String,
    input: Value,
}

const CAPSULE_TOOLS: &[&str] = &[
    "capsule_store",
    "capsule_read",
    "capsule_revise",
    "capsule_discard",
    "capsule_search",
    "capsule_compounds",
];

// ─── Punto de entrada ─────────────────────────────────────────────────────────

/// Punto de entrada del modo MCP: lee de stdin, ejecuta y escribe en stdout.
pub fn run() -> Result<()> {
    let mut raw = String::new();
    std::io::stdin().read_to_string(&mut raw)?;

    let response = match execute(&raw) {
        Ok(r) => r,
        Err(e) => Response::err("internal_error", e.to_string()),
    };

    println!("{}", serde_json::to_string(&response)?);
    Ok(())
}

/// Parsea la request, abre la DB adecuada y delega al dispatcher.
///
/// Las herramientas de capsule usan siempre la DB global; el resto usan
/// la DB resuelta por contexto (local o global).
fn execute(raw: &str) -> Result<Response> {
    let req: Request =
        serde_json::from_str(raw).map_err(|e| anyhow::anyhow!("request_parse_error: {}", e))?;

    let path = if CAPSULE_TOOLS.contains(&req.tool.as_str()) {
        config::global_db_path()
    } else {
        config::resolve_db_path()
            .ok_or_else(|| anyhow::anyhow!("no_db: no se encontró ninguna DB de Pillbox"))?
    };

    let mut conn = db::connection::open(&path)?;
    Ok(dispatch::dispatch(&mut conn, &req.tool, req.input))
}
