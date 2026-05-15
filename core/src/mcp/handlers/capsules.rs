//! Handlers MCP para la entidad Capsule.

use pillbox::{
    db::store,
    domain::{
        capsule::{self, CapsulePatch, NewCapsule},
        search::SearchParams,
    },
    error::{ContentOp, PillboxError},
};
use serde::Deserialize;
use serde_json::{json, Value};

use crate::mcp::response::{
    anyhow_to_response, check_content_size, from_pillbox, from_value, validate_input, Conn,
    Response,
};

/// Guarda una capsule nueva en la DB global.
pub fn take(conn: &mut Conn, input: Value) -> Response {
    let req: NewCapsule = match from_value(input) {
        Ok(v) => v,
        Err(r) => return r,
    };
    if let Err(r) = check_content_size(&req.content, capsule::CONTENT_MAX_CHARS, ContentOp::Create) {
        return r;
    }
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
        Ok(None) => from_pillbox(&PillboxError::CapsuleNotFound { id: req.id }),
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
    if let Some(content) = patch.content.as_deref() {
        if let Err(r) = check_content_size(content, capsule::CONTENT_MAX_CHARS, ContentOp::Update) {
            return r;
        }
    }
    if let Err(r) = validate_input(&patch) {
        return r;
    }
    match store::capsules::revise(conn, &req.id, &patch) {
        Ok(Some(c)) => Response::ok(c),
        Ok(None) => from_pillbox(&PillboxError::CapsuleNotFound { id: req.id }),
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
        Ok(None) => from_pillbox(&PillboxError::CapsuleNotFound { id: req.id }),
        Err(e) => anyhow_to_response(e),
    }
}

/// Busca capsules por texto con FTS5 y expansión fuzzy Jaro-Winkler.
pub fn search(conn: &mut Conn, input: Value) -> Response {
    #[derive(Deserialize)]
    struct In {
        query: String,
        compound: Option<String>,
        limit: Option<u32>,
        #[serde(default)]
        fuzzy: bool,
    }
    let req: In = match from_value(input) {
        Ok(v) => v,
        Err(r) => return r,
    };
    let params = SearchParams {
        query: req.query,
        bottle_id: None,
        compound: req.compound,
        limit: req.limit,
        fuzzy: req.fuzzy,
    };
    match store::search::capsule_find(conn, &params) {
        Ok(results) => Response::ok(results),
        Err(e) => anyhow_to_response(e),
    }
}

/// Lista de compounds distintos en capsules con su frecuencia.
///
/// Acepta `limit` opcional (default 50, capeado a 200). Las capsules son globales.
pub fn compounds(conn: &mut Conn, input: Value) -> Response {
    #[derive(Deserialize)]
    struct In {
        limit: Option<u32>,
    }
    let req: In = match from_value(input) {
        Ok(v) => v,
        Err(r) => return r,
    };
    let limit = req.limit.unwrap_or(50).min(200);
    match store::capsules::distinct_compounds(conn, limit) {
        Ok(rows) => {
            let entries: Vec<_> = rows
                .into_iter()
                .map(|(compound, count)| json!({ "compound": compound, "count": count }))
                .collect();
            Response::ok(entries)
        }
        Err(e) => anyhow_to_response(e),
    }
}

#[cfg(test)]
mod tests {
    use pillbox::{
        db::{connection::open_in_memory, store},
        domain::capsule::NewCapsule,
    };
    use serde_json::json;

    fn sample() -> NewCapsule {
        NewCapsule {
            title: "Test capsule".into(),
            content: "Contenido de prueba.".into(),
            compound: "convention".into(),
        }
    }

    #[test]
    fn read_resolves_12char_prefix() {
        let mut conn = open_in_memory().unwrap();
        let result = store::capsules::take(&mut conn, &sample()).unwrap();
        let short = result.id.replace('-', "").chars().take(12).collect::<String>();
        let response = super::read(&mut conn, json!({ "id": short }));
        assert!(response.ok);
        assert_eq!(response.data.unwrap()["id"].as_str().unwrap(), result.id);
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
        conn.execute(
            "INSERT INTO capsules (id, compound, title, content)
             VALUES ('01234567-aaaa-7000-8000-000000000001', 'convention', 'A', 'c')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO capsules (id, compound, title, content)
             VALUES ('01234567-aaaa-7000-8000-000000000002', 'convention', 'B', 'c')",
            [],
        )
        .unwrap();
        let response = super::read(&mut conn, json!({ "id": "01234567aaaa" }));
        assert!(!response.ok);
        assert_eq!(response.error.as_deref(), Some("ambiguous_id"));
    }

    #[test]
    fn revise_resolves_short_id() {
        let mut conn = open_in_memory().unwrap();
        let result = store::capsules::take(&mut conn, &sample()).unwrap();
        let short = result.id.replace('-', "").chars().take(12).collect::<String>();
        let response = super::revise(&mut conn, json!({ "id": short, "title": "Revisada" }));
        assert!(response.ok);
        assert_eq!(response.data.unwrap()["title"].as_str().unwrap(), "Revisada");
    }

    #[test]
    fn discard_resolves_short_id() {
        let mut conn = open_in_memory().unwrap();
        let result = store::capsules::take(&mut conn, &sample()).unwrap();
        let short = result.id.replace('-', "").chars().take(12).collect::<String>();
        let response = super::discard(&mut conn, json!({ "id": short }));
        assert!(response.ok);
    }
}
