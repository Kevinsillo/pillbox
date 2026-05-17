//! Handler HTTP para el contexto del dashboard de un bottle.

use axum::extract::{Path, Query, State};
use pillbox::db::store;
use serde::Deserialize;

use super::{conn_for_bottle_with_id, err_500, ok, ApiResponse, AppState};

fn default_8() -> u32 {
    8
}

/// Parámetros de query para ajustar el número de pills recientes devueltas.
#[derive(Deserialize)]
pub struct ContextParams {
    #[serde(default = "default_8")]
    pub pill_limit: u32,
}

/// Handler `GET /api/bottles/:id/context` — devuelve contadores y pills recientes para el dashboard.
pub async fn context_get(
    State(s): State<AppState>,
    Path(bottle_id): Path<String>,
    Query(params): Query<ContextParams>,
) -> ApiResponse {
    let (conn, full_id) = match conn_for_bottle_with_id(&s, &bottle_id) {
        Ok(v) => v,
        Err(r) => return r,
    };
    let pill_count = match store::pills::count_by_bottle(&conn, &full_id) {
        Ok(n) => n,
        Err(e) => return err_500(e),
    };
    let prescription_count = match store::prescriptions::count_by_bottle(&conn, &full_id) {
        Ok(n) => n,
        Err(e) => return err_500(e),
    };
    let recent_pills = match store::pills::list_recent(&conn, &full_id, params.pill_limit) {
        Ok(v) => v,
        Err(e) => return err_500(e),
    };
    ok(serde_json::json!({
        "context":            recent_pills,
        "pill_count":         pill_count,
        "prescription_count": prescription_count,
    }))
}
