//! Protocolo MCP (Model Context Protocol) de Pillbox — loop persistente NDJSON.
//!
//! `pillbox mcp run` arranca un proceso singleton que lee una request JSON
//! por línea en stdin, la despacha al handler correspondiente y escribe la
//! respuesta JSON (también en una línea) a stdout. El cliente TS
//! (`pillboxExec`) genera UUIDs por request y multiplexa peticiones en
//! flight; el campo `id` del protocolo permite correlacionar respuestas.
//!
//! Cada request lleva su propio `cwd` (el `process.cwd()` del cliente TS),
//! que se usa para resolver la DB local (`{cwd}/.pillbox/pillbox.db`) — el
//! proceso server es singleton y su `current_dir()` no es fiable.
//!
//! Reglas estrictas del transporte:
//!   - Líneas vacías o sólo whitespace: se ignoran silenciosamente.
//!   - JSON inválido o `cwd` ausente del payload: `id: ""`, `error: "parse_error"`
//!     (el campo `cwd` es obligatorio en el deserializador).
//!   - `cwd` vacío o no absoluto: `error: "missing_cwd"`.
//!   - stdout es SÓLO NDJSON — tracing va a stderr (configurado en `main`).
//!   - EOF en stdin: loop termina con `Ok(())`, proceso exita con 0.

use std::io::{BufRead, BufReader, Write};
use std::path::Path;

use anyhow::Result;
use serde_json::Value;

use pillbox::{config, db};

mod dispatch;
mod handlers;
pub mod response;
pub mod types;

use response::Response as InnerResponse;
use types::{Request, Response};

const CAPSULE_TOOLS: &[&str] = &[
    "capsule_store",
    "capsule_read",
    "capsule_revise",
    "capsule_discard",
    "capsule_search",
    "capsule_compounds",
];

// ─── Punto de entrada ─────────────────────────────────────────────────────────

/// Loop persistente: lee NDJSON de stdin, despacha y escribe NDJSON a stdout.
///
/// Termina con `Ok(())` al recibir EOF — el cliente cierra stdin del subproceso
/// al apagar el wrapper TS y el server debe salir limpiamente con código 0
/// en menos de un segundo (no quedan recursos pendientes).
///
/// # Errors
///
/// Sólo retorna error si `stdout.flush()` falla (pipe roto del lado cliente);
/// los errores por-request se serializan como `Response { ok: false, ... }`
/// y nunca crashean el loop.
pub fn run() -> Result<()> {
    let stdin = std::io::stdin().lock();
    let reader = BufReader::new(stdin);
    let mut stdout = std::io::stdout().lock();

    for line in reader.lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }

        let resp = handle_line(&line);
        let serialized = serde_json::to_string(&resp)?;
        stdout.write_all(serialized.as_bytes())?;
        stdout.write_all(b"\n")?;
        stdout.flush()?;
    }

    Ok(())
}

/// Procesa una línea cruda y devuelve la `Response` lista para serializar.
///
/// Encapsula la decisión de parse_error vs missing_cwd vs dispatch, así el
/// loop principal sólo se preocupa de I/O.
fn handle_line(line: &str) -> Response {
    let req: Request = match serde_json::from_str(line) {
        Ok(r) => r,
        Err(e) => {
            return Response {
                id: String::new(),
                ok: false,
                data: None,
                error: Some("parse_error".to_string()),
                message: Some(e.to_string()),
            };
        }
    };

    // Validar cwd: el struct ya lo exige como String, pero un cliente podría
    // enviarlo como "" o como ruta relativa. Bajamos a missing_cwd en ambos
    // casos — la DB local sólo es resoluble desde una ruta absoluta.
    let cwd_trim = req.cwd.trim();
    if cwd_trim.is_empty() || !Path::new(cwd_trim).is_absolute() {
        return Response {
            id: req.id,
            ok: false,
            data: None,
            error: Some("missing_cwd".to_string()),
            message: Some("request.cwd must be an absolute path".to_string()),
        };
    }

    let cwd_path = Path::new(cwd_trim);
    let inner = execute_tool(&req.tool, req.input, cwd_path);
    into_public(req.id, inner)
}

/// Convierte la `response::Response` interna (sin id) a la pública (con id).
fn into_public(id: String, inner: InnerResponse) -> Response {
    Response {
        id,
        ok: inner.ok,
        data: inner.data,
        error: inner.error,
        message: inner.message,
    }
}

/// Resuelve la DB adecuada para `tool` desde el `cwd` del cliente y despacha.
///
/// Las herramientas de capsule usan siempre la DB global; el resto resuelven
/// con `resolve_db_path(cwd)` — local del proyecto si existe, global si no.
fn execute_tool(tool: &str, input: Value, cwd: &Path) -> InnerResponse {
    let (path, scope) = if CAPSULE_TOOLS.contains(&tool) {
        (config::global_db_path(), db::DbScope::Global)
    } else {
        let Some(p) = config::resolve_db_path(cwd) else {
            return InnerResponse::err("no_db", "no Pillbox DB found");
        };
        let scope = if p == config::global_db_path() {
            db::DbScope::Global
        } else {
            db::DbScope::Local
        };
        (p, scope)
    };

    let mut conn = match db::connection::open(&path, scope) {
        Ok(c) => c,
        Err(e) => return InnerResponse::err("internal_error", e.to_string()),
    };
    dispatch::dispatch(&mut conn, tool, input)
}
