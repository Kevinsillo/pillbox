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

/// Aplica un patch parcial sobre una pill existente.
///
/// # Errors
///
/// Retorna error si la validación del patch falla o si la actualización en la base de datos falla.
pub fn revise(conn: &mut Conn, input: Value) -> Response {
    #[derive(Deserialize)]
    struct In {
        id: i64,
        patch: PillPatch,
    }
    let req: In = match from_value(input) {
        Ok(v) => v,
        Err(r) => return r,
    };
    if let Err(r) = validate_input(&req.patch) {
        return r;
    }
    match store::pills::revise(conn, req.id, &req.patch) {
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

/// Genera un bloque de contexto con pills y prescripciones recientes de un bottle.
///
/// Retorna un JSON con `context` (texto ensamblado), `prescription_count` y `pill_count`.
/// Los límites `prescription_limit` (defecto 5) y `pill_limit` (defecto 30) permiten
/// ajustar cuántos registros se incluyen en el contexto generado.
///
/// # Errors
///
/// Retorna error si la consulta a la base de datos falla.
pub fn context(conn: &mut Conn, input: Value) -> Response {
    #[derive(Deserialize)]
    struct In {
        bottle_id: String,
        #[serde(default = "default_5")]
        prescription_limit: u32,
        #[serde(default = "default_30")]
        pill_limit: u32,
    }
    fn default_5() -> u32 {
        5
    }
    fn default_30() -> u32 {
        30
    }

    let req: In = match from_value(input) {
        Ok(v) => v,
        Err(r) => return r,
    };
    match store::search::pill_context(conn, &req.bottle_id, req.prescription_limit, req.pill_limit)
    {
        Ok(ctx) => Response::ok(json!({
            "context":            ctx.context,
            "prescription_count": ctx.prescription_count,
            "pill_count":         ctx.pill_count,
        })),
        Err(e) => anyhow_to_response(e),
    }
}
