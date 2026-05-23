//! Handlers MCP para la entidad Prescription.

use pillbox::{db::store, domain::prescription::NewPrescription, error::PillboxError};
use serde::Deserialize;
use serde_json::{json, Value};

use crate::mcp::response::{
    anyhow_to_response, from_pillbox, from_value, validate_input, Conn, Response,
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
        Ok(Some(mut rx)) => match store::counters::increment_views(conn, "prescriptions", &rx.id) {
            Ok(v) => {
                rx.views = v;
                Response::ok(rx)
            }
            Err(e) => anyhow_to_response(e),
        },
        Ok(None) => from_pillbox(&PillboxError::PrescriptionNotFound { id: req.id }),
        Err(e) => anyhow_to_response(e),
    }
}

/// Reabre una prescripción cerrada (limpia `ended_at`).
///
/// Idempotente: si la prescription ya está abierta, devuelve `Ok` con la rx
/// sin modificar timestamps. Múltiples prescriptions abiertas por bottle son
/// válidas, así que reabrir nunca colisiona con otras rx activas del mismo
/// bottle.
///
/// # Errors
///
/// Retorna error si la prescription no existe o está descartada
/// (`deleted_at IS NOT NULL`).
pub fn reopen(conn: &mut Conn, input: Value) -> Response {
    #[derive(Deserialize)]
    struct In {
        id: String,
    }
    let req: In = match from_value(input) {
        Ok(v) => v,
        Err(r) => return r,
    };
    if req.id.trim().is_empty() {
        return Response::err("invalid_input", "id must not be empty");
    }
    match store::prescriptions::reopen(conn, &req.id) {
        Ok(rx) => Response::ok(rx),
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
        db::{connection::open_in_memory, store, DbScope},
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
        let mut conn = open_in_memory(DbScope::Local).unwrap();
        let bottle_id = make_bottle(&mut conn, "mcp-rx-read");
        let rx_id = make_rx(&mut conn, &bottle_id, "Test session");
        let short = rx_id.replace('-', "").chars().take(12).collect::<String>();
        let response = super::read(&mut conn, json!({ "id": short }));
        assert!(response.ok);
        assert_eq!(response.data.unwrap()["id"].as_str().unwrap(), rx_id);
    }

    #[test]
    fn close_resolves_short_id() {
        let mut conn = open_in_memory(DbScope::Local).unwrap();
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
        let mut conn = open_in_memory(DbScope::Local).unwrap();
        let bottle_id = make_bottle(&mut conn, "mcp-rx-discard");
        let rx_id = make_rx(&mut conn, &bottle_id, "Discard session");
        let short = rx_id.replace('-', "").chars().take(12).collect::<String>();
        let response = super::discard(&mut conn, json!({ "id": short }));
        assert!(response.ok);
    }

    #[test]
    fn read_returns_invalid_id_when_too_short() {
        let mut conn = open_in_memory(DbScope::Local).unwrap();
        let response = super::read(&mut conn, json!({ "id": "abc" }));
        assert!(!response.ok);
        assert_eq!(response.error.as_deref(), Some("invalid_id"));
    }

    #[test]
    fn mcp_reopen_success() {
        let mut conn = open_in_memory(DbScope::Local).unwrap();
        let bottle_id = make_bottle(&mut conn, "mcp-reopen-ok");
        let rx_id = make_rx(&mut conn, &bottle_id, "Reabrir");
        store::prescriptions::close(&mut conn, &rx_id).unwrap();

        let response = super::reopen(&mut conn, json!({ "id": rx_id.clone() }));
        assert!(
            response.ok,
            "expected ok response, got {:?}",
            response.error
        );
        let data = response.data.unwrap();
        assert_eq!(data["id"].as_str().unwrap(), rx_id);
        assert!(data["ended_at"].is_null());
    }

    /// Tras eliminar la unicidad de "una rx abierta por bottle", reabrir una rx
    /// cerrada cuando coexiste otra abierta en el mismo bottle ya no es una
    /// colisión: el handler debe devolver Ok con la rx reabierta.
    #[test]
    fn mcp_reopen_with_another_open_in_same_bottle_succeeds() {
        let mut conn = open_in_memory(DbScope::Local).unwrap();
        let bottle_id = make_bottle(&mut conn, "mcp-reopen-multi-open");

        let rx_a = make_rx(&mut conn, &bottle_id, "A");
        store::prescriptions::close(&mut conn, &rx_a).unwrap();

        let rx_b = make_rx(&mut conn, &bottle_id, "B");

        let response = super::reopen(&mut conn, json!({ "id": rx_a.clone() }));
        assert!(
            response.ok,
            "expected ok response, got {:?}",
            response.error
        );
        let data = response.data.unwrap();
        assert_eq!(data["id"].as_str().unwrap(), rx_a);
        assert!(data["ended_at"].is_null());
        // rx_b sigue abierta.
        let b = store::prescriptions::read(&conn, &rx_b).unwrap().unwrap();
        assert!(b.ended_at.is_none());
    }

    #[test]
    fn mcp_reopen_empty_id() {
        let mut conn = open_in_memory(DbScope::Local).unwrap();
        let response = super::reopen(&mut conn, json!({ "id": "" }));
        assert!(!response.ok);
        assert_eq!(response.error.as_deref(), Some("invalid_input"));
    }

    /// (3) prescription_read incrementa prescriptions.views: 0 → 1 → 2.
    #[test]
    fn prescription_read_increments_views() {
        let mut conn = open_in_memory(DbScope::Local).unwrap();
        let bottle_id = make_bottle(&mut conn, "mcp-rx-views");
        let rx_id = make_rx(&mut conn, &bottle_id, "Views test");

        let views0: i64 = conn
            .query_row(
                "SELECT views FROM prescriptions WHERE id = ?1",
                params![rx_id],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(views0, 0);

        let r1 = super::read(&mut conn, json!({ "id": rx_id.clone() }));
        assert!(r1.ok);
        assert_eq!(r1.data.unwrap()["views"].as_i64().unwrap(), 1);

        let views1: i64 = conn
            .query_row(
                "SELECT views FROM prescriptions WHERE id = ?1",
                params![rx_id],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(views1, 1);

        let r2 = super::read(&mut conn, json!({ "id": rx_id.clone() }));
        assert_eq!(r2.data.unwrap()["views"].as_i64().unwrap(), 2);
    }

    #[test]
    fn read_returns_ambiguous_id_error() {
        let mut conn = open_in_memory(DbScope::Local).unwrap();
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
