//! Handlers MCP para la entidad Prescription.

use pillbox::{db::store, domain::prescription::NewPrescription};
use serde::Deserialize;
use serde_json::{json, Value};

use crate::mcp::response::{
    anyhow_to_response, from_value, not_found, validate_input, Conn, Response,
};

/// Abre una prescripción nueva en la DB y la devuelve serializada.
///
/// # Errors
///
/// Retorna error si la validación falla o si la inserción en la base de datos falla.
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

/// Cierra una prescripción activa por su ID de cadena.
///
/// # Errors
///
/// Retorna error si la consulta a la base de datos falla.
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

/// Lee una prescripción activa por su ID de cadena.
///
/// # Errors
///
/// Retorna error si la consulta a la base de datos falla. Retorna `not_found` si la prescripción no existe.
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

/// Realiza un soft delete en cascada de una prescripción y sus pills asociadas.
///
/// # Errors
///
/// Retorna error si la consulta a la base de datos falla.
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
