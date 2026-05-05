//! Handler HTTP para generar el bloque de contexto de un bottle.

use axum::extract::{Path, Query, State};
use pillbox::db::store;
use serde::Deserialize;
use serde_json::json;

use super::{conn_for_bottle, default_30, err_500, ok, ApiResponse, AppState};

/// Parámetros de query para ajustar el límite del contexto generado.
#[derive(Deserialize)]
pub struct ContextParams {
    #[serde(default = "default_30")]
    pub limit: u32,
}

/// Handler `GET /api/bottles/:id/context` — devuelve el índice de prescriptions del bottle.
pub async fn context_get(
    State(s): State<AppState>,
    Path(bottle_id): Path<String>,
    Query(params): Query<ContextParams>,
) -> ApiResponse {
    let conn = match conn_for_bottle(&s, &bottle_id) {
        Ok(c) => c,
        Err(r) => return r,
    };
    let ctx = match store::search::bottle_context(&conn, &bottle_id, params.limit) {
        Ok(c) => c,
        Err(e) => return err_500(e),
    };
    ok(json!({
        "context":            ctx.context,
        "prescription_count": ctx.prescription_count,
    }))
}
