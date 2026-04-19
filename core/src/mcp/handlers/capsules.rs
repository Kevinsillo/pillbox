use pillbox::{
    db::store,
    domain::capsule::{CapsulePatch, NewCapsule},
};
use serde::Deserialize;
use serde_json::Value;

use crate::mcp::response::{anyhow_to_response, from_value, not_found, validate_input, Conn, Response};

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

pub fn read(conn: &mut Conn, input: Value) -> Response {
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

pub fn revise(conn: &mut Conn, input: Value) -> Response {
    #[derive(Deserialize)]
    struct In {
        id: i64,
        patch: CapsulePatch,
    }
    let req: In = match from_value(input) {
        Ok(v) => v,
        Err(r) => return r,
    };
    if let Err(r) = validate_input(&req.patch) {
        return r;
    }
    match store::capsules::revise(conn, req.id, &req.patch) {
        Ok(Some(c)) => Response::ok(c),
        Ok(None) => not_found("capsule", req.id),
        Err(e) => anyhow_to_response(e),
    }
}

pub fn discard(conn: &mut Conn, input: Value) -> Response {
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
