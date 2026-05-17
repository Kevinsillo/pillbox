//! Handlers MCP para la entidad Pill.

use pillbox::{
    db::store,
    domain::{
        pill::{self, NewPill, PillPatch},
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

/// Resuelve un `bottle_id` opcional recibido por MCP (puede llegar como id
/// corto de 12 chars sin guiones) al UUID canónico almacenado en la DB.
///
/// Devuelve `Ok(None)` si la entrada es `None`. Devuelve `Err(Response)` con
/// `bottle_not_found` si el id no resuelve, o el error nativo si es inválido
/// o ambiguo.
fn resolve_bottle_filter(
    conn: &Conn,
    bottle_id: Option<String>,
) -> std::result::Result<Option<String>, Response> {
    let Some(id) = bottle_id else {
        return Ok(None);
    };
    match store::id_resolver::resolve_id(conn, "bottles", &id) {
        Ok(Some(canonical)) => Ok(Some(canonical)),
        Ok(None) => Err(Response::err(
            "bottle_not_found",
            format!("no bottle matches id '{}'", id),
        )),
        Err(e) => Err(anyhow_to_response(e)),
    }
}

/// Crea una pill nueva a partir de los datos del input y la persiste en la DB.
///
/// # Errors
///
/// Retorna error si la validación falla o si la inserción en la base de datos falla.
pub fn take(conn: &mut Conn, input: Value) -> Response {
    let req: NewPill = match from_value(input) {
        Ok(v) => v,
        Err(r) => return r,
    };
    if let Err(r) = check_content_size(&req.content, pill::CONTENT_MAX_CHARS, ContentOp::Create) {
        return r;
    }
    if let Err(r) = validate_input(&req) {
        return r;
    }
    match store::pills::take(conn, &req) {
        Ok(r) => Response::ok(r),
        Err(e) => anyhow_to_response(e),
    }
}

/// Lee una pill activa por su ID numérico.
///
/// # Errors
///
/// Retorna error si la consulta a la base de datos falla. Retorna `not_found` si la pill no existe.
pub fn read(conn: &mut Conn, input: Value) -> Response {
    #[derive(Deserialize)]
    struct In {
        id: String,
    }
    let req: In = match from_value(input) {
        Ok(v) => v,
        Err(r) => return r,
    };
    match store::pills::read(conn, &req.id) {
        Ok(Some(mut p)) => {
            // Tras una lectura exitosa, incrementar el contador de vistas y
            // reflejar el nuevo valor en la respuesta (sin segunda query).
            match store::counters::increment_views(conn, "pills", &p.id) {
                Ok(v) => {
                    p.views = v;
                    Response::ok(p)
                }
                Err(e) => anyhow_to_response(e),
            }
        }
        Ok(None) => from_pillbox(&PillboxError::PillNotFound { id: req.id }),
        Err(e) => anyhow_to_response(e),
    }
}

/// Aplica un patch parcial sobre una pill existente.
///
/// # Errors
///
/// Retorna error si la validación del patch falla o si la actualización en la base de datos falla.
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
    let patch = PillPatch {
        title: req.title,
        content: req.content,
        compound: req.compound,
    };
    if let Some(content) = patch.content.as_deref() {
        if let Err(r) = check_content_size(content, pill::CONTENT_MAX_CHARS, ContentOp::Update) {
            return r;
        }
    }
    if let Err(r) = validate_input(&patch) {
        return r;
    }
    match store::pills::revise(conn, &req.id, &patch) {
        Ok(Some(p)) => Response::ok(p),
        Ok(None) => from_pillbox(&PillboxError::PillNotFound { id: req.id }),
        Err(e) => anyhow_to_response(e),
    }
}

/// Realiza un soft delete de una pill por su ID numérico.
///
/// # Errors
///
/// Retorna error si la consulta a la base de datos falla. Retorna `not_found` si la pill no existe.
pub fn discard(conn: &mut Conn, input: Value) -> Response {
    #[derive(Deserialize)]
    struct In {
        id: String,
    }
    let req: In = match from_value(input) {
        Ok(v) => v,
        Err(r) => return r,
    };
    match store::pills::discard(conn, &req.id) {
        Ok(Some(r)) => Response::ok(r),
        Ok(None) => from_pillbox(&PillboxError::PillNotFound { id: req.id }),
        Err(e) => anyhow_to_response(e),
    }
}

/// Busca pills mediante FTS5 con expansión fuzzy Jaro-Winkler.
///
/// # Errors
///
/// Retorna error si la búsqueda en la base de datos falla.
pub fn search(conn: &mut Conn, input: Value) -> Response {
    #[derive(Deserialize)]
    struct In {
        #[serde(default)]
        query: Option<String>,
        bottle_id: Option<String>,
        compound: Option<String>,
        limit: Option<u32>,
        #[serde(default)]
        fuzzy: bool,
    }
    let req: In = match from_value(input) {
        Ok(v) => v,
        Err(r) => return r,
    };
    let bottle_id = match resolve_bottle_filter(conn, req.bottle_id) {
        Ok(v) => v,
        Err(r) => return r,
    };
    let params = SearchParams {
        query: req.query.unwrap_or_default(),
        bottle_id,
        compound: req.compound,
        fuzzy: req.fuzzy,
    };
    let pagination = pillbox::domain::PaginationParams {
        page: 1,
        page_size: req.limit.unwrap_or(20).min(100),
    };
    match store::search::pill_find(conn, &params, &pagination) {
        Ok(page) => Response::ok(page.items),
        Err(e) => anyhow_to_response(e),
    }
}

/// Lista de compounds distintos con su frecuencia.
///
/// Acepta `bottle_id` opcional para filtrar a un bottle concreto, y `limit`
/// opcional (default 50, capeado a 200).
///
/// # Errors
///
/// Retorna error si la consulta a la base de datos falla.
pub fn compounds(conn: &mut Conn, input: Value) -> Response {
    #[derive(Deserialize)]
    struct In {
        bottle_id: Option<String>,
        limit: Option<u32>,
    }
    let req: In = match from_value(input) {
        Ok(v) => v,
        Err(r) => return r,
    };
    let bottle_id = match resolve_bottle_filter(conn, req.bottle_id) {
        Ok(v) => v,
        Err(r) => return r,
    };
    let limit = req.limit.unwrap_or(50).min(200);
    match store::pills::distinct_compounds(conn, bottle_id.as_deref(), limit) {
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

/// Índice navegable de prescriptions de un bottle.
///
/// # Errors
///
/// Retorna error si la consulta a la base de datos falla.
pub fn bottle_context(conn: &mut Conn, input: Value) -> Response {
    #[derive(Deserialize)]
    struct In {
        bottle_id: String,
        #[serde(default = "default_30")]
        limit: u32,
    }
    fn default_30() -> u32 {
        30
    }

    let req: In = match from_value(input) {
        Ok(v) => v,
        Err(r) => return r,
    };
    match store::search::bottle_context(conn, &req.bottle_id, req.limit) {
        Ok(ctx) => {
            let views = match store::id_resolver::resolve_id(conn, "bottles", &req.bottle_id) {
                Ok(Some(bid)) => match store::counters::increment_views(conn, "bottles", &bid) {
                    Ok(v) => Some(v),
                    Err(e) => return anyhow_to_response(e),
                },
                Ok(None) => None,
                Err(e) => return anyhow_to_response(e),
            };
            let mut prescriptions = Vec::with_capacity(ctx.prescriptions.len());
            for rx in ctx.prescriptions {
                let rx_views = match store::counters::increment_views(conn, "prescriptions", &rx.id)
                {
                    Ok(v) => v,
                    Err(e) => return anyhow_to_response(e),
                };
                prescriptions.push(json!({
                    "id":         rx.id,
                    "title":      rx.title,
                    "started_at": rx.started_at,
                    "ended_at":   rx.ended_at,
                    "pill_count": rx.pill_count,
                    "views":      rx_views,
                }));
            }
            let mut payload = json!({
                "prescription_count": ctx.prescription_count,
                "prescriptions":      prescriptions,
            });
            if let Some(v) = views {
                payload["views"] = json!(v);
            }
            Response::ok(payload)
        }
        Err(e) => anyhow_to_response(e),
    }
}

/// Pills de una prescription concreta con snippets navegables.
///
/// # Errors
///
/// Retorna error si la búsqueda en la base de datos falla.
pub fn prescription_context(conn: &mut Conn, input: Value) -> Response {
    #[derive(Deserialize)]
    struct In {
        prescription_id: String,
        #[serde(default = "default_30")]
        limit: u32,
    }
    fn default_30() -> u32 {
        30
    }

    let req: In = match from_value(input) {
        Ok(v) => v,
        Err(r) => return r,
    };
    match store::search::prescription_context(conn, &req.prescription_id, req.limit) {
        Ok(ctx) => {
            let views = if let Some(rx_id) = ctx.id.as_deref() {
                match store::counters::increment_views(conn, "prescriptions", rx_id) {
                    Ok(v) => Some(v),
                    Err(e) => return anyhow_to_response(e),
                }
            } else {
                None
            };
            let mut pills = Vec::with_capacity(ctx.pills.len());
            for p in ctx.pills {
                let p_views = match store::counters::increment_views(conn, "pills", &p.id) {
                    Ok(v) => v,
                    Err(e) => return anyhow_to_response(e),
                };
                pills.push(json!({
                    "id":       p.id,
                    "compound": p.compound,
                    "title":    p.title,
                    "snippet":  p.snippet,
                    "views":    p_views,
                }));
            }
            let mut payload = json!({
                "id":         ctx.id,
                "title":      ctx.title,
                "started_at": ctx.started_at,
                "ended_at":   ctx.ended_at,
                "pill_count": ctx.pill_count,
                "pills":      pills,
            });
            if let Some(v) = views {
                payload["views"] = json!(v);
            }
            Response::ok(payload)
        }
        Err(e) => anyhow_to_response(e),
    }
}

#[cfg(test)]
mod tests {
    use pillbox::{
        db::{connection::open_in_memory, store, DbScope},
        domain::{
            bottle::{BottleScope, NewBottle},
            pill::NewPill,
            prescription::NewPrescription,
        },
    };
    use rusqlite::params;
    use serde_json::json;

    fn setup(conn: &mut rusqlite::Connection) -> (String, String) {
        let bottle = store::bottles::create(
            conn,
            &NewBottle {
                name: "mcp-pills-test".into(),
                display_name: "MCP Pills Test".into(),
                directory: "/tmp/mcp-pills-test".into(),
                scope: BottleScope::Local,
            },
        )
        .unwrap();
        let rx = store::prescriptions::open(
            conn,
            &NewPrescription {
                bottle_id: bottle.id.clone(),
                title: "Test session".into(),
                author_name: None,
                author_email: None,
            },
        )
        .unwrap();
        (bottle.id, rx.id)
    }

    fn make_pill(conn: &mut rusqlite::Connection, rx_id: &str) -> String {
        store::pills::take(
            conn,
            &NewPill {
                title: "Test pill".into(),
                content: "Contenido de prueba.".into(),
                compound: "decision".into(),
                prescription_id: rx_id.to_string(),
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
        let (_, rx_id) = setup(&mut conn);
        let pill_id = make_pill(&mut conn, &rx_id);
        let short = pill_id
            .replace('-', "")
            .chars()
            .take(12)
            .collect::<String>();
        let response = super::read(&mut conn, json!({ "id": short }));
        assert!(response.ok);
        assert_eq!(response.data.unwrap()["id"].as_str().unwrap(), pill_id);
    }

    #[test]
    fn read_returns_invalid_id_when_too_short() {
        let mut conn = open_in_memory(DbScope::Local).unwrap();
        let response = super::read(&mut conn, json!({ "id": "abc" }));
        assert!(!response.ok);
        assert_eq!(response.error.as_deref(), Some("invalid_id"));
    }

    #[test]
    fn read_returns_ambiguous_id_error() {
        let mut conn = open_in_memory(DbScope::Local).unwrap();
        let (_, rx_id) = setup(&mut conn);
        conn.execute(
            "INSERT INTO pills (id, compound, title, content, prescription_id)
             VALUES ('01234567-aaaa-7000-8000-000000000001', 'decision', 'A', 'c', ?1)",
            params![rx_id],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO pills (id, compound, title, content, prescription_id)
             VALUES ('01234567-aaaa-7000-8000-000000000002', 'decision', 'B', 'c', ?1)",
            params![rx_id],
        )
        .unwrap();
        let response = super::read(&mut conn, json!({ "id": "01234567aaaa" }));
        assert!(!response.ok);
        assert_eq!(response.error.as_deref(), Some("ambiguous_id"));
    }

    #[test]
    fn revise_resolves_short_id() {
        let mut conn = open_in_memory(DbScope::Local).unwrap();
        let (_, rx_id) = setup(&mut conn);
        let pill_id = make_pill(&mut conn, &rx_id);
        let short = pill_id
            .replace('-', "")
            .chars()
            .take(12)
            .collect::<String>();
        let response = super::revise(&mut conn, json!({ "id": short, "title": "Revisada" }));
        assert!(response.ok);
        assert_eq!(
            response.data.unwrap()["title"].as_str().unwrap(),
            "Revisada"
        );
    }

    #[test]
    fn discard_resolves_short_id() {
        let mut conn = open_in_memory(DbScope::Local).unwrap();
        let (_, rx_id) = setup(&mut conn);
        let pill_id = make_pill(&mut conn, &rx_id);
        let short = pill_id
            .replace('-', "")
            .chars()
            .take(12)
            .collect::<String>();
        let response = super::discard(&mut conn, json!({ "id": short }));
        assert!(response.ok);
    }

    /// Lee `views` desde una tabla concreta vía SQL directo, para verificar
    /// el estado en la DB sin pasar por handlers.
    fn db_views(conn: &rusqlite::Connection, table: &str, id: &str) -> i64 {
        let sql = format!("SELECT views FROM {table} WHERE id = ?1");
        conn.query_row(&sql, rusqlite::params![id], |r| r.get::<_, i64>(0))
            .unwrap()
    }

    // ──────────────────────────────────────────────────────────────────────
    // Phase 5 — Contador de vistas: tests de integración
    // ──────────────────────────────────────────────────────────────────────

    /// (1) pill_read incrementa pills.views: 0 → 1 → 2 (en respuesta y en DB).
    #[test]
    fn pill_read_increments_views() {
        let mut conn = open_in_memory(DbScope::Local).unwrap();
        let (_, rx_id) = setup(&mut conn);
        let pill_id = make_pill(&mut conn, &rx_id);
        assert_eq!(db_views(&conn, "pills", &pill_id), 0);

        let r1 = super::read(&mut conn, json!({ "id": pill_id.clone() }));
        assert!(r1.ok);
        assert_eq!(r1.data.unwrap()["views"].as_i64().unwrap(), 1);
        assert_eq!(db_views(&conn, "pills", &pill_id), 1);

        let r2 = super::read(&mut conn, json!({ "id": pill_id.clone() }));
        assert_eq!(r2.data.unwrap()["views"].as_i64().unwrap(), 2);
        assert_eq!(db_views(&conn, "pills", &pill_id), 2);
    }

    /// (4) prescription_context incrementa prescriptions.views Y las pills
    /// anidadas dentro.
    #[test]
    fn prescription_context_increments_prescription_and_nested_pills() {
        let mut conn = open_in_memory(DbScope::Local).unwrap();
        let (_, rx_id) = setup(&mut conn);
        let pill_a = make_pill(&mut conn, &rx_id);
        let pill_b = make_pill(&mut conn, &rx_id);

        assert_eq!(db_views(&conn, "prescriptions", &rx_id), 0);
        assert_eq!(db_views(&conn, "pills", &pill_a), 0);
        assert_eq!(db_views(&conn, "pills", &pill_b), 0);

        let resp =
            super::prescription_context(&mut conn, json!({ "prescription_id": rx_id.clone() }));
        assert!(resp.ok, "expected ok, got {:?}", resp.error);
        let data = resp.data.unwrap();
        assert_eq!(data["views"].as_i64().unwrap(), 1);

        assert_eq!(db_views(&conn, "prescriptions", &rx_id), 1);
        assert_eq!(db_views(&conn, "pills", &pill_a), 1);
        assert_eq!(db_views(&conn, "pills", &pill_b), 1);
        for pill in data["pills"].as_array().unwrap() {
            assert_eq!(pill["views"].as_i64().unwrap(), 1);
        }
    }

    /// (5) bottle_context incrementa bottles.views Y las prescriptions
    /// anidadas dentro.
    #[test]
    fn bottle_context_increments_bottle_and_nested_prescriptions() {
        let mut conn = open_in_memory(DbScope::Local).unwrap();
        let (bottle_id, rx_id_a) = setup(&mut conn);
        // Cerramos rx_a para poder abrir otra en el mismo bottle.
        store::prescriptions::close(&mut conn, &rx_id_a).unwrap();
        let rx_b = store::prescriptions::open(
            &mut conn,
            &pillbox::domain::prescription::NewPrescription {
                bottle_id: bottle_id.clone(),
                title: "Second".into(),
                author_name: None,
                author_email: None,
            },
        )
        .unwrap();

        assert_eq!(db_views(&conn, "bottles", &bottle_id), 0);
        assert_eq!(db_views(&conn, "prescriptions", &rx_id_a), 0);
        assert_eq!(db_views(&conn, "prescriptions", &rx_b.id), 0);

        let resp = super::bottle_context(&mut conn, json!({ "bottle_id": bottle_id.clone() }));
        assert!(resp.ok, "expected ok, got {:?}", resp.error);
        let data = resp.data.unwrap();
        assert_eq!(data["views"].as_i64().unwrap(), 1);

        assert_eq!(db_views(&conn, "bottles", &bottle_id), 1);
        assert_eq!(db_views(&conn, "prescriptions", &rx_id_a), 1);
        assert_eq!(db_views(&conn, "prescriptions", &rx_b.id), 1);
        for rx in data["prescriptions"].as_array().unwrap() {
            assert_eq!(rx["views"].as_i64().unwrap(), 1);
        }
    }

    /// (6) Negativa: store::pills::read directo NO incrementa views.
    #[test]
    fn store_pills_read_does_not_increment_views() {
        let mut conn = open_in_memory(DbScope::Local).unwrap();
        let (_, rx_id) = setup(&mut conn);
        let pill_id = make_pill(&mut conn, &rx_id);
        assert_eq!(db_views(&conn, "pills", &pill_id), 0);

        let p = store::pills::read(&conn, &pill_id).unwrap().unwrap();
        assert_eq!(p.views, 0);
        assert_eq!(db_views(&conn, "pills", &pill_id), 0);
    }

    /// (7) Negativa: MCP pill_search no incrementa views de las pills devueltas.
    #[test]
    fn pill_search_does_not_increment_views() {
        let mut conn = open_in_memory(DbScope::Local).unwrap();
        let (_, rx_id) = setup(&mut conn);
        let pill_id = make_pill(&mut conn, &rx_id);
        assert_eq!(db_views(&conn, "pills", &pill_id), 0);

        let resp = super::search(&mut conn, json!({ "query": "prueba" }));
        assert!(resp.ok, "expected ok, got {:?}", resp.error);
        // Independientemente del resultado, no debe haberse incrementado.
        assert_eq!(db_views(&conn, "pills", &pill_id), 0);
    }

    /// `pill_search` debe aceptar `compound` sin `query` (lista por compound).
    #[test]
    fn pill_search_accepts_compound_without_query() {
        let mut conn = open_in_memory(DbScope::Local).unwrap();
        let (_, rx_id) = setup(&mut conn);
        let _ = make_pill(&mut conn, &rx_id);

        let resp = super::search(&mut conn, json!({ "compound": "decision" }));
        assert!(resp.ok, "expected ok, got {:?}", resp.error);
    }

    fn short_id(id: &str) -> String {
        id.replace('-', "").chars().take(12).collect()
    }

    /// Regresión: `pill_search` con `bottle_id` corto (formato MCP) debe
    /// encontrar las pills del bottle. Antes del fix devolvía 0 porque el
    /// handler no resolvía el id corto a UUID canónico antes del WHERE.
    #[test]
    fn pill_search_resolves_short_bottle_id() {
        let mut conn = open_in_memory(DbScope::Local).unwrap();
        let (bottle_id, rx_id) = setup(&mut conn);
        let _ = make_pill(&mut conn, &rx_id);

        let short = short_id(&bottle_id);
        let resp = super::search(&mut conn, json!({ "query": "prueba", "bottle_id": short }));
        assert!(resp.ok, "expected ok, got {:?}", resp.error);
        let items = resp.data.unwrap();
        let arr = items.as_array().expect("expected array");
        assert_eq!(arr.len(), 1, "expected 1 result with short bottle_id");
    }

    /// Regresión: `pill_search` con `bottle_id` que no resuelve devuelve
    /// `bottle_not_found` (en lugar del antiguo "0 resultados silencioso").
    #[test]
    fn pill_search_bottle_not_found() {
        let mut conn = open_in_memory(DbScope::Local).unwrap();
        let (_, rx_id) = setup(&mut conn);
        let _ = make_pill(&mut conn, &rx_id);

        let resp = super::search(
            &mut conn,
            json!({ "query": "prueba", "bottle_id": "deadbeefdead" }),
        );
        assert!(!resp.ok);
        assert_eq!(resp.error.as_deref(), Some("bottle_not_found"));
    }

    /// Regresión: `pill_compounds` con `bottle_id` corto debe agregar las
    /// pills del bottle. Antes del fix devolvía vacío.
    #[test]
    fn pill_compounds_resolves_short_bottle_id() {
        let mut conn = open_in_memory(DbScope::Local).unwrap();
        let (bottle_id, rx_id) = setup(&mut conn);
        let _ = make_pill(&mut conn, &rx_id);

        let short = short_id(&bottle_id);
        let resp = super::compounds(&mut conn, json!({ "bottle_id": short }));
        assert!(resp.ok, "expected ok, got {:?}", resp.error);
        let arr = resp.data.unwrap();
        let arr = arr.as_array().expect("expected array");
        assert!(!arr.is_empty(), "expected at least one compound");
    }
}
