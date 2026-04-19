use pillbox::{db::store, domain::prescription::NewPrescription};
use serde::Deserialize;
use serde_json::{json, Value};

use crate::mcp::response::{anyhow_to_response, from_value, not_found, validate_input, Conn, Response};

pub fn open(conn: &mut Conn, input: Value) -> Response {
    let req: NewPrescription = match from_value(input) {
        Ok(v) => v,
        Err(r) => return r,
    };
    if let Err(r) = validate_input(&req) {
        return r;
    }
    match store::prescriptions::open(conn, &req) {
        Ok(rx) => Response::ok(rx),
        Err(e) => anyhow_to_response(e),
    }
}

pub fn close(conn: &mut Conn, input: Value) -> Response {
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

pub fn read(conn: &mut Conn, input: Value) -> Response {
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

pub fn discard(conn: &mut Conn, input: Value) -> Response {
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
