//! Dispatcher `pillbox exec`.
//!
//! Lee un payload JSON de stdin, ejecuta la operación sobre la DB y escribe
//! el resultado en stdout. Es el mecanismo de comunicación con el servidor MCP.
//!
//! Protocolo:
//!   stdin  → `{ "tool": "<nombre>", "input": { ... } }`
//!   stdout → `{ "ok": true,  "data": { ... } }`            (éxito)
//!   stdout → `{ "ok": false, "error": "<code>", "message": "<msg>" }` (error)
//!
//! Errores estructurados (como `prescription_already_open`) incluyen además
//! un campo `data` con los detalles necesarios para que el modelo decida.

use std::io::Read;

use anyhow::Result;
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use validator::Validate;

use pillbox::{
    config,
    db::{
        self,
        store::{self, PrescriptionAlreadyOpen},
    },
    domain::{
        bottle::NewBottle,
        capsule::{CapsulePatch, NewCapsule},
        pill::{NewPill, PillPatch},
        prescription::NewPrescription,
        search::SearchParams,
    },
};

// ─── Protocolo ────────────────────────────────────────────────────────────────

#[derive(Deserialize)]
struct Request {
    tool: String,
    input: Value,
}

#[derive(Serialize)]
struct Response {
    ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    data: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    message: Option<String>,
}

impl Response {
    fn ok(data: impl Serialize) -> Self {
        Self {
            ok: true,
            data: serde_json::to_value(data).ok(),
            error: None,
            message: None,
        }
    }

    fn err(error: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            ok: false,
            data: None,
            error: Some(error.into()),
            message: Some(message.into()),
        }
    }

    fn err_with_data(error: impl Into<String>, message: impl Into<String>, data: Value) -> Self {
        Self {
            ok: false,
            data: Some(data),
            error: Some(error.into()),
            message: Some(message.into()),
        }
    }
}

// ─── Punto de entrada ─────────────────────────────────────────────────────────

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

const CAPSULE_TOOLS: &[&str] = &[
    "capsule_take",
    "capsule_read",
    "capsule_revise",
    "capsule_discard",
    "capsule_search",
    "capsule_compounds",
];

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
    Ok(dispatch(&mut conn, &req.tool, req.input))
}

// ─── Dispatcher ───────────────────────────────────────────────────────────────

fn dispatch(conn: &mut Connection, tool: &str, input: Value) -> Response {
    match tool {
        // ── Pills ──────────────────────────────────────────────────────────────
        "pill_take" => {
            let req: NewPill = match from_value(input) {
                Ok(v) => v,
                Err(r) => return r,
            };
            if let Err(r) = validate(&req) {
                return r;
            }
            match store::pills::take(conn, &req) {
                Ok(r) => Response::ok(r),
                Err(e) => anyhow_to_response(e),
            }
        }

        "pill_read" => {
            #[derive(Deserialize)]
            struct In {
                id: i64,
            }
            let req: In = match from_value(input) {
                Ok(v) => v,
                Err(r) => return r,
            };
            match store::pills::read(conn, req.id) {
                Ok(Some(p)) => Response::ok(p),
                Ok(None) => not_found("pill", req.id),
                Err(e) => anyhow_to_response(e),
            }
        }

        "pill_revise" => {
            #[derive(Deserialize)]
            struct In {
                id: i64,
                patch: PillPatch,
            }
            let req: In = match from_value(input) {
                Ok(v) => v,
                Err(r) => return r,
            };
            if let Err(r) = validate(&req.patch) {
                return r;
            }
            match store::pills::revise(conn, req.id, &req.patch) {
                Ok(Some(p)) => Response::ok(p),
                Ok(None) => not_found("pill", req.id),
                Err(e) => anyhow_to_response(e),
            }
        }

        "pill_discard" => {
            #[derive(Deserialize)]
            struct In {
                id: i64,
            }
            let req: In = match from_value(input) {
                Ok(v) => v,
                Err(r) => return r,
            };
            match store::pills::discard(conn, req.id) {
                Ok(Some(r)) => Response::ok(r),
                Ok(None) => not_found("pill", req.id),
                Err(e) => anyhow_to_response(e),
            }
        }

        "pill_search" => {
            let params: SearchParams = match from_value(input) {
                Ok(v) => v,
                Err(r) => return r,
            };
            match store::search::pill_find(conn, &params) {
                Ok(results) => Response::ok(results),
                Err(e) => anyhow_to_response(e),
            }
        }

        "pill_context" => {
            #[derive(Deserialize)]
            struct In {
                bottle_id: i64,
                #[serde(default = "default_5")]
                prescription_limit: u32,
                #[serde(default = "default_30")]
                pill_limit: u32,
            }
            let req: In = match from_value(input) {
                Ok(v) => v,
                Err(r) => return r,
            };
            match store::search::pill_context(
                conn,
                req.bottle_id,
                req.prescription_limit,
                req.pill_limit,
            ) {
                Ok(ctx) => Response::ok(json!({
                    "context":             ctx.context,
                    "prescription_count":  ctx.prescription_count,
                    "pill_count":          ctx.pill_count,
                })),
                Err(e) => anyhow_to_response(e),
            }
        }

        // ── Capsules ───────────────────────────────────────────────────────────
        "capsule_take" => {
            let req: NewCapsule = match from_value(input) {
                Ok(v) => v,
                Err(r) => return r,
            };
            if let Err(r) = validate(&req) {
                return r;
            }
            match store::capsules::take(conn, &req) {
                Ok(r) => Response::ok(r),
                Err(e) => anyhow_to_response(e),
            }
        }

        "capsule_read" => {
            #[derive(Deserialize)]
            struct In {
                id: i64,
            }
            let req: In = match from_value(input) {
                Ok(v) => v,
                Err(r) => return r,
            };
            match store::capsules::read(conn, req.id) {
                Ok(Some(c)) => Response::ok(c),
                Ok(None) => not_found("capsule", req.id),
                Err(e) => anyhow_to_response(e),
            }
        }

        "capsule_revise" => {
            #[derive(Deserialize)]
            struct In {
                id: i64,
                patch: CapsulePatch,
            }
            let req: In = match from_value(input) {
                Ok(v) => v,
                Err(r) => return r,
            };
            if let Err(r) = validate(&req.patch) {
                return r;
            }
            match store::capsules::revise(conn, req.id, &req.patch) {
                Ok(Some(c)) => Response::ok(c),
                Ok(None) => not_found("capsule", req.id),
                Err(e) => anyhow_to_response(e),
            }
        }

        "capsule_discard" => {
            #[derive(Deserialize)]
            struct In {
                id: i64,
            }
            let req: In = match from_value(input) {
                Ok(v) => v,
                Err(r) => return r,
            };
            match store::capsules::discard(conn, req.id) {
                Ok(Some(r)) => Response::ok(r),
                Ok(None) => not_found("capsule", req.id),
                Err(e) => anyhow_to_response(e),
            }
        }

        "capsule_search" => {
            #[derive(Deserialize)]
            struct In {
                query: String,
                compound: Option<String>,
                limit: Option<u32>,
            }
            let req: In = match from_value(input) {
                Ok(v) => v,
                Err(r) => return r,
            };
            match store::search::capsule_find(conn, &req.query, req.compound.as_deref(), req.limit)
            {
                Ok(results) => Response::ok(results),
                Err(e) => anyhow_to_response(e),
            }
        }

        // ── Prescriptions ──────────────────────────────────────────────────────
        "prescription_open" => {
            let req: NewPrescription = match from_value(input) {
                Ok(v) => v,
                Err(r) => return r,
            };
            if let Err(r) = validate(&req) {
                return r;
            }
            match store::prescriptions::open(conn, &req) {
                Ok(rx) => Response::ok(rx),
                Err(e) => {
                    if let Some(already) = e.downcast_ref::<PrescriptionAlreadyOpen>() {
                        Response::err_with_data(
                            "prescription_already_open",
                            already.to_string(),
                            serde_json::to_value(already).unwrap_or(Value::Null),
                        )
                    } else {
                        anyhow_to_response(e)
                    }
                }
            }
        }

        "prescription_close" => {
            #[derive(Deserialize)]
            struct In {
                id: String,
            }
            let req: In = match from_value(input) {
                Ok(v) => v,
                Err(r) => return r,
            };
            match store::prescriptions::close(conn, &req.id) {
                Ok(rx) => Response::ok(rx),
                Err(e) => anyhow_to_response(e),
            }
        }

        "prescription_read" => {
            #[derive(Deserialize)]
            struct In {
                id: String,
            }
            let req: In = match from_value(input) {
                Ok(v) => v,
                Err(r) => return r,
            };
            match store::prescriptions::read(conn, &req.id) {
                Ok(Some(rx)) => Response::ok(rx),
                Ok(None) => not_found("prescription", &req.id),
                Err(e) => anyhow_to_response(e),
            }
        }

        "prescription_discard" => {
            #[derive(Deserialize)]
            struct In {
                id: String,
            }
            let req: In = match from_value(input) {
                Ok(v) => v,
                Err(r) => return r,
            };
            match store::prescriptions::discard(conn, &req.id) {
                Ok(()) => Response::ok(json!({ "discarded": true })),
                Err(e) => anyhow_to_response(e),
            }
        }

        // ── Compounds ─────────────────────────────────────────────────────────
        "pill_compounds" | "capsule_compounds" => {
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

        // ── Bottles ────────────────────────────────────────────────────────────
        "bottle_create" => {
            let req: NewBottle = match from_value(input) {
                Ok(v) => v,
                Err(r) => return r,
            };
            if let Err(r) = validate(&req) {
                return r;
            }
            match store::bottles::create(conn, &req) {
                Ok(b) => Response::ok(b),
                Err(e) => anyhow_to_response(e),
            }
        }

        "bottle_list" => match store::bottles::list(conn) {
            Ok(bottles) => Response::ok(bottles),
            Err(e) => anyhow_to_response(e),
        },

        // ── Desconocido ────────────────────────────────────────────────────────
        other => Response::err(
            "unknown_tool",
            format!("herramienta desconocida: '{}'", other),
        ),
    }
}

// ─── Helpers ──────────────────────────────────────────────────────────────────

fn not_found(entity: &str, id: impl std::fmt::Display) -> Response {
    Response::err("not_found", format!("{} {} no encontrada", entity, id))
}

fn from_value<T: for<'de> Deserialize<'de>>(v: Value) -> Result<T, Response> {
    serde_json::from_value(v).map_err(|e| Response::err("invalid_input", e.to_string()))
}

fn validate<T: Validate>(v: &T) -> Result<(), Response> {
    v.validate()
        .map_err(|e| Response::err("validation_error", e.to_string()))
}

/// Convierte un error `anyhow` en una `Response` de error.
/// Si el mensaje sigue el patrón `"code: message"`, extrae el código.
fn anyhow_to_response(e: anyhow::Error) -> Response {
    let msg = e.to_string();
    if let Some(colon) = msg.find(':') {
        let code = msg[..colon].trim();
        if code.chars().all(|c| c.is_alphanumeric() || c == '_') {
            let message = msg[colon + 1..].trim().to_string();
            return Response::err(code, message);
        }
    }
    Response::err("error", msg)
}

fn default_5() -> u32 {
    5
}
fn default_30() -> u32 {
    30
}
