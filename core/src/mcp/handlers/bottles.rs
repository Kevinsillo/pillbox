use pillbox::{db::store, domain::bottle::NewBottle};
use serde_json::Value;

use crate::mcp::response::{anyhow_to_response, from_value, validate_input, Conn, Response};

pub fn create(conn: &mut Conn, input: Value) -> Response {
    let req: NewBottle = match from_value(input) {
        Ok(v) => v,
        Err(r) => return r,
    };
    if let Err(r) = validate_input(&req) {
        return r;
    }
    match store::bottles::create(conn, &req) {
        Ok(b) => Response::ok(b),
        Err(e) => anyhow_to_response(e),
    }
}

pub fn list(conn: &mut Conn, _input: Value) -> Response {
    match store::bottles::list(conn) {
        Ok(bottles) => Response::ok(bottles),
        Err(e) => anyhow_to_response(e),
    }
}
