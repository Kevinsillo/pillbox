use axum::extract::{Path, Query, State};
use pillbox::db::store;
use serde::Deserialize;
use serde_json::json;

use super::{conn_for_bottle, default_5, default_30, err_500, ok, ApiResponse, AppState};

#[derive(Deserialize)]
pub struct ContextParams {
    #[serde(default = "default_5")]
    pub prescription_limit: u32,
    #[serde(default = "default_30")]
    pub pill_limit: u32,
}

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
