//! Handlers MCP para la entidad Pill.

use pillbox::{
    db::store,
    domain::{
        pill::{NewPill, PillPatch},
        search::SearchParams,
    },
};
use serde::Deserialize;
use serde_json::{json, Value};

use crate::mcp::response::{
    anyhow_to_response, from_value, not_found, validate_input, Conn, Response,
};

/// Crea una pill nueva a partir de los datos del input y la persiste en la DB.
///
/// # Errors
///
/// Retorna error si la validación falla o si la inserción en la base de datos falla.
pub fn take(conn: &mut Conn, input: Value) -> Response {
    let req: NewPill = match from_value(input) {
        Ok(v) => v,
        Err(r) => return r,
    };
    if let Err(r) = validate_input(&req) {
        return r;
    }
    match store::pills::take(conn, &req) {
        Ok(r) => Response::ok(r),
        Err(e) => anyhow_to_response(e),
    }
}

/// Lee una pill activa por su ID numérico.
///
/// # Errors
///
/// Retorna error si la consulta a la base de datos falla. Retorna `not_found` si la pill no existe.
pub fn read(conn: &mut Conn, input: Value) -> Response {
    #[derive(Deserialize)]
    struct In {
        id: String,
    }
    let req: In = match from_value(input) {
        Ok(v) => v,
        Err(r) => return r,
    };
    match store::pills::read(conn, &req.id) {
        Ok(Some(p)) => Response::ok(p),
        Ok(None) => not_found("pill", req.id),
        Err(e) => anyhow_to_response(e),
    }
}

/// Aplica un patch parcial sobre una pill existente.
///
/// # Errors
///
/// Retorna error si la validación del patch falla o si la actualización en la base de datos falla.
pub fn revise(conn: &mut Conn, input: Value) -> Response {
    #[derive(Deserialize)]
    struct In {
        id: String,
        title: Option<String>,
        content: Option<String>,
        compound: Option<String>,
    }
    let req: In = match from_value(input) {
        Ok(v) => v,
        Err(r) => return r,
    };
    let patch = PillPatch { title: req.title, content: req.content, compound: req.compound };
    if let Err(r) = validate_input(&patch) {
        return r;
    }
    match store::pills::revise(conn, &req.id, &patch) {
        Ok(Some(p)) => Response::ok(p),
        Ok(None) => not_found("pill", req.id),
        Err(e) => anyhow_to_response(e),
    }
}

/// Realiza un soft delete de una pill por su ID numérico.
///
/// # Errors
///
/// Retorna error si la consulta a la base de datos falla. Retorna `not_found` si la pill no existe.
pub fn discard(conn: &mut Conn, input: Value) -> Response {
    #[derive(Deserialize)]
    struct In {
        id: String,
    }
    let req: In = match from_value(input) {
        Ok(v) => v,
        Err(r) => return r,
    };
    match store::pills::discard(conn, &req.id) {
        Ok(Some(r)) => Response::ok(r),
        Ok(None) => not_found("pill", req.id),
        Err(e) => anyhow_to_response(e),
    }
}

/// Busca pills mediante FTS5 con expansión fuzzy Jaro-Winkler.
///
/// # Errors
///
/// Retorna error si la búsqueda en la base de datos falla.
pub fn search(conn: &mut Conn, input: Value) -> Response {
    let params: SearchParams = match from_value(input) {
        Ok(v) => v,
        Err(r) => return r,
    };
    match store::search::pill_find(conn, &params) {
        Ok(results) => Response::ok(results),
        Err(e) => anyhow_to_response(e),
    }
}

/// Índice navegable de prescriptions de un bottle.
///
/// # Errors
///
/// Retorna error si la consulta a la base de datos falla.
pub fn bottle_context(conn: &mut Conn, input: Value) -> Response {
    #[derive(Deserialize)]
    struct In {
        bottle_id: String,
        #[serde(default = "default_30")]
        limit: u32,
    }
    fn default_30() -> u32 {
        30
    }

    let req: In = match from_value(input) {
        Ok(v) => v,
        Err(r) => return r,
    };
    match store::search::bottle_context(conn, &req.bottle_id, req.limit) {
        Ok(ctx) => {
            let prescriptions: Vec<_> = ctx.prescriptions.into_iter().map(|rx| json!({
                "id":         rx.id,
                "title":      rx.title,
                "started_at": rx.started_at,
                "ended_at":   rx.ended_at,
                "pill_count": rx.pill_count,
            })).collect();
            Response::ok(json!({
                "prescription_count": ctx.prescription_count,
                "prescriptions":      prescriptions,
            }))
        }
        Err(e) => anyhow_to_response(e),
    }
}

/// Pills de una prescription concreta con snippets navegables.
///
/// # Errors
///
/// Retorna error si la consulta a la base de datos falla.
pub fn prescription_context(conn: &mut Conn, input: Value) -> Response {
    #[derive(Deserialize)]
    struct In {
        prescription_id: String,
        #[serde(default = "default_30")]
        limit: u32,
    }
    fn default_30() -> u32 {
        30
    }

    let req: In = match from_value(input) {
        Ok(v) => v,
        Err(r) => return r,
    };
    match store::search::prescription_context(conn, &req.prescription_id, req.limit) {
        Ok(ctx) => {
            let pills: Vec<_> = ctx.pills.into_iter().map(|p| json!({
                "id":       p.id,
                "compound": p.compound,
                "title":    p.title,
                "snippet":  p.snippet,
            })).collect();
            Response::ok(json!({
                "id":         ctx.id,
                "title":      ctx.title,
                "started_at": ctx.started_at,
                "ended_at":   ctx.ended_at,
                "pill_count": ctx.pill_count,
                "pills":      pills,
            }))
        }
        Err(e) => anyhow_to_response(e),
    }
}
