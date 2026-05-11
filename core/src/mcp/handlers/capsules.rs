//! Handlers MCP para la entidad Capsule.

use pillbox::{
    db::store,
    domain::capsule::{CapsulePatch, NewCapsule},
};
use serde::Deserialize;
use serde_json::Value;

use crate::mcp::response::{
    anyhow_to_response, from_value, not_found, validate_input, Conn, Response,
};

/// Guarda una capsule nueva en la DB global.
pub fn take(conn: &mut Conn, input: Value) -> Response {
    let req: NewCapsule = match from_value(input) {
        Ok(v) => v,
        Err(r) => return r,
    };
    if let Err(r) = validate_input(&req) {
        return r;
    }
    match store::capsules::take(conn, &req) {
        Ok(r) => Response::ok(r),
        Err(e) => anyhow_to_response(e),
    }
}

/// Lee una capsule activa por ID.
pub fn read(conn: &mut Conn, input: Value) -> Response {
    #[derive(Deserialize)]
    struct In {
        id: String,
    }
    let req: In = match from_value(input) {
        Ok(v) => v,
        Err(r) => return r,
    };
    match store::capsules::read(conn, &req.id) {
        Ok(Some(c)) => Response::ok(c),
        Ok(None) => not_found("capsule", req.id),
        Err(e) => anyhow_to_response(e),
    }
}

/// Actualiza campos de una capsule existente (patch parcial).
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
    let patch = CapsulePatch { title: req.title, content: req.content, compound: req.compound };
    if let Err(r) = validate_input(&patch) {
        return r;
    }
    match store::capsules::revise(conn, &req.id, &patch) {
        Ok(Some(c)) => Response::ok(c),
        Ok(None) => not_found("capsule", req.id),
        Err(e) => anyhow_to_response(e),
    }
}

/// Soft delete de una capsule por ID.
pub fn discard(conn: &mut Conn, input: Value) -> Response {
    #[derive(Deserialize)]
    struct In {
        id: String,
    }
    let req: In = match from_value(input) {
        Ok(v) => v,
        Err(r) => return r,
    };
    match store::capsules::discard(conn, &req.id) {
        Ok(Some(r)) => Response::ok(r),
        Ok(None) => not_found("capsule", req.id),
        Err(e) => anyhow_to_response(e),
    }
}

/// Busca capsules por texto con FTS5 y expansión fuzzy.
pub fn search(conn: &mut Conn, input: Value) -> Response {
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
    match store::search::capsule_find(conn, &req.query, req.compound.as_deref(), req.limit) {
        Ok(results) => Response::ok(results),
        Err(e) => anyhow_to_response(e),
    }
}
