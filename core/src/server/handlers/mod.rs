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
use pillbox::db::store::registered_bottles;

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

pub(super) fn err_with_context(
    status: StatusCode,
    error: &str,
    context: serde_json::Value,
) -> ApiResponse {
    let mut body = json!({ "ok": false, "error": error });
    if let (Some(obj), Some(extra)) = (body.as_object_mut(), context.as_object()) {
        for (k, v) in extra {
            obj.insert(k.clone(), v.clone());
        }
    }
    ApiResponse(status, body)
}

pub(super) fn err_422(e: impl ToString) -> ApiResponse {
    err(
        StatusCode::UNPROCESSABLE_ENTITY,
        "validation_error",
        &e.to_string(),
    )
}

pub(super) fn err_404_bottle(id: &str) -> ApiResponse {
    err_with_context(StatusCode::NOT_FOUND, "bottle_not_found", json!({ "bottle_id": id }))
}

pub(super) fn err_404_registered_bottle(id: &str) -> ApiResponse {
    err_with_context(StatusCode::NOT_FOUND, "registered_bottle_not_found", json!({ "reg_id": id }))
}

pub(super) fn err_404_pill(id: i64) -> ApiResponse {
    err_with_context(StatusCode::NOT_FOUND, "pill_not_found", json!({ "pill_id": id }))
}

pub(super) fn err_404_prescription(id: &str) -> ApiResponse {
    err_with_context(StatusCode::NOT_FOUND, "prescription_not_found", json!({ "prescription_id": id }))
}

pub(super) fn err_404_capsule(id: i64) -> ApiResponse {
    err_with_context(StatusCode::NOT_FOUND, "capsule_not_found", json!({ "capsule_id": id }))
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

/// Abre la DB local del bottle identificado por UUID.
/// Busca la ruta en registered_bottles de la DB global.
pub(super) fn conn_for_bottle(s: &AppState, bottle_id: &str) -> Result<rusqlite::Connection, ApiResponse> {
    let global = db::connection::open(&s.global_db_path).map_err(err_500)?;
    let reg = registered_bottles::find_by_bottle_id(&global, bottle_id)
        .map_err(err_500)?
        .ok_or_else(|| err_404_bottle(bottle_id))?;
    db::connection::open_existing(std::path::Path::new(&reg.db_path)).map_err(err_500)
}

pub(super) fn open_global_conn(state: &AppState) -> Result<rusqlite::Connection, ApiResponse> {
    db::connection::open(&state.global_db_path).map_err(err_500)
}

pub(super) fn default_5() -> u32 { 5 }
pub(super) fn default_30() -> u32 { 30 }
pub(super) fn default_50() -> u32 { 50 }
