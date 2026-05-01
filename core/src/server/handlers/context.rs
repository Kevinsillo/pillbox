//! Handler HTTP para generar el bloque de contexto de un bottle.

use axum::extract::{Path, Query, State};
use pillbox::db::store;
use serde::Deserialize;
use serde_json::json;

use super::{conn_for_bottle, default_30, default_5, err_500, ok, ApiResponse, AppState};

/// Parámetros de query para ajustar los límites del contexto generado.
#[derive(Deserialize)]
pub struct ContextParams {
    #[serde(default = "default_5")]
    pub prescription_limit: u32,
    #[serde(default = "default_30")]
    pub pill_limit: u32,
}

/// Handler `GET /api/bottles/:id/context` — devuelve pills recientes y contadores del bottle.
pub async fn context_get(
    State(s): State<AppState>,
    Path(bottle_id): Path<String>,
    Query(params): Query<ContextParams>,
) -> ApiResponse {
    let conn = match conn_for_bottle(&s, &bottle_id) {
        Ok(c) => c,
        Err(r) => return r,
    };
    let ctx = match store::search::pill_context(
        &conn,
        &bottle_id,
        params.prescription_limit,
        params.pill_limit,
    ) {
        Ok(c) => c,
        Err(e) => return err_500(e),
    };
    let pills = match store::search::recent_pills(&conn, &bottle_id, params.pill_limit) {
        Ok(p) => p,
        Err(e) => return err_500(e),
    };
    ok(json!({
        "context":            pills,
        "prescription_count": ctx.prescription_count,
        "pill_count":         ctx.pill_count,
    }))
}
