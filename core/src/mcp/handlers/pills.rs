use pillbox::{
    db::store,
    domain::{
        pill::{NewPill, PillPatch},
        search::SearchParams,
    },
};
use serde::Deserialize;
use serde_json::{json, Value};

use crate::mcp::response::{anyhow_to_response, from_value, not_found, validate_input, Conn, Response};

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

pub fn context(conn: &mut Conn, input: Value) -> Response {
    #[derive(Deserialize)]
    struct In {
        bottle_id: String,
        #[serde(default = "default_5")]
        prescription_limit: u32,
        #[serde(default = "default_30")]
        pill_limit: u32,
    }
    fn default_5() -> u32 { 5 }
    fn default_30() -> u32 { 30 }

    let req: In = match from_value(input) {
        Ok(v) => v,
        Err(r) => return r,
    };
    match store::search::pill_context(conn, &req.bottle_id, req.prescription_limit, req.pill_limit) {
        Ok(ctx) => Response::ok(json!({
            "context":            ctx.context,
            "prescription_count": ctx.prescription_count,
            "pill_count":         ctx.pill_count,
        })),
        Err(e) => anyhow_to_response(e),
    }
}
