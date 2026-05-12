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

#[cfg(test)]
mod tests {
    use pillbox::{
        db::{connection::open_in_memory, store},
        domain::{
            bottle::{BottleScope, NewBottle},
            prescription::NewPrescription,
        },
    };
    use rusqlite::params;
    use serde_json::json;

    fn make_bottle(conn: &mut rusqlite::Connection, name: &str) -> String {
        store::bottles::create(
            conn,
            &NewBottle {
                name: name.into(),
                display_name: name.into(),
                directory: format!("/tmp/{}", name),
                scope: BottleScope::Local,
            },
        )
        .unwrap()
        .id
    }

    fn make_rx(conn: &mut rusqlite::Connection, bottle_id: &str, title: &str) -> String {
        store::prescriptions::open(
            conn,
            &NewPrescription {
                bottle_id: bottle_id.into(),
                title: title.into(),
                author_name: None,
                author_email: None,
            },
        )
        .unwrap()
        .id
    }

    #[test]
    fn read_resolves_12char_prefix() {
        let mut conn = open_in_memory().unwrap();
        let bottle_id = make_bottle(&mut conn, "mcp-rx-read");
        let rx_id = make_rx(&mut conn, &bottle_id, "Test session");
        let short = rx_id.replace('-', "").chars().take(12).collect::<String>();
        let response = super::read(&mut conn, json!({ "id": short }));
        assert!(response.ok);
        assert_eq!(response.data.unwrap()["id"].as_str().unwrap(), rx_id);
    }

    #[test]
    fn close_resolves_short_id() {
        let mut conn = open_in_memory().unwrap();
        let bottle_id = make_bottle(&mut conn, "mcp-rx-close");
        let rx_id = make_rx(&mut conn, &bottle_id, "Close session");
        let short = rx_id.replace('-', "").chars().take(12).collect::<String>();
        let response = super::close(&mut conn, json!({ "id": short }));
        assert!(response.ok);
        let ended_at = &response.data.unwrap()["ended_at"];
        assert!(!ended_at.is_null());
    }

    #[test]
    fn discard_resolves_short_id() {
        let mut conn = open_in_memory().unwrap();
        let bottle_id = make_bottle(&mut conn, "mcp-rx-discard");
        let rx_id = make_rx(&mut conn, &bottle_id, "Discard session");
        let short = rx_id.replace('-', "").chars().take(12).collect::<String>();
        let response = super::discard(&mut conn, json!({ "id": short }));
        assert!(response.ok);
    }

    #[test]
    fn read_returns_invalid_id_when_too_short() {
        let mut conn = open_in_memory().unwrap();
        let response = super::read(&mut conn, json!({ "id": "abc" }));
        assert!(!response.ok);
        assert_eq!(response.error.as_deref(), Some("invalid_id"));
    }

    #[test]
    fn read_returns_ambiguous_id_error() {
        let mut conn = open_in_memory().unwrap();
        let bottle_id = make_bottle(&mut conn, "mcp-rx-amb");
        // Primera rx cerrada para no violar el índice UNIQUE parcial (ended_at IS NULL).
        conn.execute(
            "INSERT INTO prescriptions (id, bottle_id, title, ended_at)
             VALUES ('01234567-aaaa-7000-8000-000000000001', ?1, 'A', datetime('now'))",
            params![bottle_id],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO prescriptions (id, bottle_id, title)
             VALUES ('01234567-aaaa-7000-8000-000000000002', ?1, 'B')",
            params![bottle_id],
        )
        .unwrap();
        let response = super::read(&mut conn, json!({ "id": "01234567aaaa" }));
        assert!(!response.ok);
        assert_eq!(response.error.as_deref(), Some("ambiguous_id"));
    }
}
