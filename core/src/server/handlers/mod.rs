mod bottles;
mod capsules;
mod context;
mod pills;
mod prescriptions;

pub use bottles::*;
pub use capsules::*;
pub use context::*;
pub use pills::*;
pub use prescriptions::*;

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;
use serde_json::json;

use pillbox::db;

use super::AppState;

// ─── Tipo de respuesta uniforme ───────────────────────────────────────────────

pub(super) struct ApiResponse(pub StatusCode, pub serde_json::Value);

impl IntoResponse for ApiResponse {
    fn into_response(self) -> Response {
        (self.0, Json(self.1)).into_response()
    }
}

pub(super) fn ok(data: impl Serialize) -> ApiResponse {
    ApiResponse(StatusCode::OK, json!({ "ok": true, "data": data }))
}

pub(super) fn ok_created(data: impl Serialize) -> ApiResponse {
    ApiResponse(StatusCode::CREATED, json!({ "ok": true, "data": data }))
}

pub(super) async fn version_get() -> ApiResponse {
    ok(json!({ "version": env!("CARGO_PKG_VERSION") }))
}

pub(super) fn err(status: StatusCode, error: &str, message: &str) -> ApiResponse {
    ApiResponse(
        status,
        json!({ "ok": false, "error": error, "message": message }),
    )
}

pub(super) fn err_422(e: impl ToString) -> ApiResponse {
    err(
        StatusCode::UNPROCESSABLE_ENTITY,
        "validation_error",
        &e.to_string(),
    )
}

pub(super) fn err_404(what: &str, id: impl std::fmt::Display) -> ApiResponse {
    err(
        StatusCode::NOT_FOUND,
        "not_found",
        &format!("{} {} no encontrado", what, id),
    )
}

pub(super) fn err_500(e: anyhow::Error) -> ApiResponse {
    err(StatusCode::INTERNAL_SERVER_ERROR, "error", &e.to_string())
}

pub(super) fn err_409(error: &str, message: &str, data: serde_json::Value) -> ApiResponse {
    ApiResponse(
        StatusCode::CONFLICT,
        json!({ "ok": false, "error": error, "message": message, "data": data }),
    )
}

pub(super) fn open_conn(state: &AppState) -> Result<rusqlite::Connection, ApiResponse> {
    db::connection::open(&state.db_path).map_err(err_500)
}

pub(super) fn open_global_conn(state: &AppState) -> Result<rusqlite::Connection, ApiResponse> {
    db::connection::open(&state.global_db_path).map_err(err_500)
}

pub(super) fn default_5() -> u32 { 5 }
pub(super) fn default_30() -> u32 { 30 }
pub(super) fn default_50() -> u32 { 50 }
