//! Handler HTTP para el contexto del dashboard de un bottle.
//!
//! Las queries rusqlite van envueltas en `spawn_blocking`.

use crate::db::store;
use crate::db::store::registered_bottles;
use crate::error::PillboxError;
use axum::extract::{Path, Query, State};
use serde::Deserialize;

use super::{
    blocking, conn_for_bottle_with_id, err_400_invalid_id, err_404_bottle, err_409_ambiguous_id,
    err_500, ok, open_global_conn, ApiResponse, AppState,
};

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
    blocking(move || {
        // Resolve registered bottle to obtain the on-disk DB path for size metadata.
        let db_size_bytes: u64 = match open_global_conn(&s) {
            Ok(global) => match registered_bottles::find_by_bottle_id(&global, &bottle_id) {
                Ok(Some(reg)) => std::fs::metadata(&reg.db_path)
                    .map(|m| m.len())
                    .unwrap_or(0),
                Ok(None) => return err_404_bottle(&bottle_id),
                Err(e) => match e.downcast::<PillboxError>() {
                    Ok(PillboxError::AmbiguousId {
                        ref id_prefix,
                        ref candidates,
                    }) => return err_409_ambiguous_id(id_prefix, candidates),
                    Ok(PillboxError::InvalidId { ref id }) => return err_400_invalid_id(id),
                    Ok(other) => return err_500(other.into()),
                    Err(e) => return err_500(e),
                },
            },
            Err(r) => return r,
        };

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
        let open_prescription_count =
            match store::prescriptions::count_open_by_bottle(&conn, &full_id) {
                Ok(n) => n,
                Err(e) => return err_500(e),
            };
        let recent_pills = match store::pills::list_recent(&conn, &full_id, params.pill_limit) {
            Ok(v) => v,
            Err(e) => return err_500(e),
        };
        ok(serde_json::json!({
            "context":                 recent_pills,
            "pill_count":              pill_count,
            "prescription_count":      prescription_count,
            "open_prescription_count": open_prescription_count,
            "db_size_bytes":           db_size_bytes,
        }))
    })
    .await
}
