use axum::extract::{Query, State};
use pillbox::db::store;
use serde::Deserialize;
use serde_json::json;

use super::{default_5, default_30, err_500, ok, open_conn, ApiResponse, AppState};

#[derive(Deserialize)]
pub struct ContextParams {
    pub bottle_id: i64,
    #[serde(default = "default_5")]
    pub prescription_limit: u32,
    #[serde(default = "default_30")]
    pub pill_limit: u32,
}

pub async fn context_get(
    State(s): State<AppState>,
    Query(params): Query<ContextParams>,
) -> ApiResponse {
    let conn = match open_conn(&s) {
        Ok(c) => c,
        Err(r) => return r,
    };
    let ctx = match store::search::pill_context(
        &conn,
        params.bottle_id,
        params.prescription_limit,
        params.pill_limit,
    ) {
        Ok(c) => c,
        Err(e) => return err_500(e),
    };
    let pills = match store::search::recent_pills(&conn, params.bottle_id, params.pill_limit) {
        Ok(p) => p,
        Err(e) => return err_500(e),
    };
    ok(json!({
        "context":            pills,
        "prescription_count": ctx.prescription_count,
        "pill_count":         ctx.pill_count,
    }))
}
