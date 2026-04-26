use pillbox::{config, db, db::store, db::store::registered_bottles, domain::bottle::NewBottle};
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
        Ok(b) => {
            if let Some(local_path) = config::resolve_db_path() {
                let global_path = config::global_db_path();
                if let Ok(global_conn) = db::connection::open(&global_path) {
                    let _ = registered_bottles::register(
                        &global_conn,
                        &b.id,
                        &b.name,
                        &b.display_name,
                        &local_path.to_string_lossy(),
                    );
                }
            }
            Response::ok(b)
        }
        Err(e) => anyhow_to_response(e),
    }
}

pub fn list(conn: &mut Conn, _input: Value) -> Response {
    match store::bottles::list(conn) {
        Ok(bottles) => Response::ok(bottles),
        Err(e) => anyhow_to_response(e),
    }
}
