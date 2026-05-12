//! Handlers HTTP de la REST API y utilidades de respuesta compartidas.

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
use pillbox::error::PillboxError;

use super::AppState;

// ─── Tipo de respuesta uniforme ───────────────────────────────────────────────

/// Respuesta HTTP uniforme: par `(StatusCode, JSON)` que implementa `IntoResponse`.
pub(super) struct ApiResponse(pub StatusCode, pub serde_json::Value);

impl IntoResponse for ApiResponse {
    fn into_response(self) -> Response {
        (self.0, Json(self.1)).into_response()
    }
}

/// Construye una respuesta 200 OK con el payload serializado en `data`.
pub(super) fn ok(data: impl Serialize) -> ApiResponse {
    ApiResponse(StatusCode::OK, json!({ "ok": true, "data": data }))
}

/// Construye una respuesta 201 Created con el payload serializado en `data`.
pub(super) fn ok_created(data: impl Serialize) -> ApiResponse {
    ApiResponse(StatusCode::CREATED, json!({ "ok": true, "data": data }))
}

/// Handler `GET /api/version` — devuelve la versión del binario.
pub(super) async fn version_get() -> ApiResponse {
    ok(json!({ "version": env!("CARGO_PKG_VERSION") }))
}

/// Construye una respuesta de error con `status`, código `error` y mensaje legible.
pub(super) fn err(status: StatusCode, error: &str, message: &str) -> ApiResponse {
    ApiResponse(
        status,
        json!({ "ok": false, "error": error, "message": message }),
    )
}

/// Construye una respuesta de error enriquecida con campos adicionales de contexto.
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

/// Respuesta 422 Unprocessable Entity para errores de validación de entrada.
pub(super) fn err_422(e: impl ToString) -> ApiResponse {
    err(
        StatusCode::UNPROCESSABLE_ENTITY,
        "validation_error",
        &e.to_string(),
    )
}

/// Respuesta 404 para un bottle no encontrado por `bottle_id`.
pub(super) fn err_404_bottle(id: &str) -> ApiResponse {
    err_with_context(
        StatusCode::NOT_FOUND,
        "bottle_not_found",
        json!({ "bottle_id": id }),
    )
}

/// Respuesta 404 para un registered_bottle no encontrado por `reg_id`.
pub(super) fn err_404_registered_bottle(id: &str) -> ApiResponse {
    err_with_context(
        StatusCode::NOT_FOUND,
        "registered_bottle_not_found",
        json!({ "reg_id": id }),
    )
}

/// Respuesta 404 para una pill no encontrada por `pill_id`.
pub(super) fn err_404_pill(id: impl std::fmt::Display) -> ApiResponse {
    err_with_context(
        StatusCode::NOT_FOUND,
        "pill_not_found",
        json!({ "pill_id": id.to_string() }),
    )
}

/// Respuesta 404 para una prescription no encontrada por `prescription_id`.
pub(super) fn err_404_prescription(id: &str) -> ApiResponse {
    err_with_context(
        StatusCode::NOT_FOUND,
        "prescription_not_found",
        json!({ "prescription_id": id }),
    )
}

/// Respuesta 404 para una capsule no encontrada por `capsule_id`.
pub(super) fn err_404_capsule(id: impl std::fmt::Display) -> ApiResponse {
    err_with_context(
        StatusCode::NOT_FOUND,
        "capsule_not_found",
        json!({ "capsule_id": id.to_string() }),
    )
}

/// Respuesta 500 Internal Server Error envolviendo un error de anyhow.
pub(super) fn err_500(e: anyhow::Error) -> ApiResponse {
    err(StatusCode::INTERNAL_SERVER_ERROR, "error", &e.to_string())
}

/// Respuesta 409 Conflict con código de error, mensaje y datos adicionales.
pub(super) fn err_409(error: &str, message: &str, data: serde_json::Value) -> ApiResponse {
    ApiResponse(
        StatusCode::CONFLICT,
        json!({ "ok": false, "error": error, "message": message, "data": data }),
    )
}

/// Respuesta 400 para un ID demasiado corto (menos de 8 chars).
pub(super) fn err_400_invalid_id(id: &str) -> ApiResponse {
    err_with_context(
        StatusCode::BAD_REQUEST,
        "invalid_id",
        json!({ "id": id, "message": "El ID debe tener al menos 8 caracteres" }),
    )
}

/// Respuesta 409 para un prefijo de ID ambiguo (coincide con >1 registro).
pub(super) fn err_409_ambiguous_id(prefix: &str, candidates: &[String]) -> ApiResponse {
    err_with_context(
        StatusCode::CONFLICT,
        "ambiguous_id",
        json!({ "id_prefix": prefix, "candidates": candidates }),
    )
}

/// Abre la DB local del bottle identificado por UUID.
/// Busca la ruta en registered_bottles de la DB global.
pub(super) fn conn_for_bottle(
    s: &AppState,
    bottle_id: &str,
) -> Result<rusqlite::Connection, ApiResponse> {
    let global = db::connection::open(&s.global_db_path).map_err(err_500)?;
    let reg = registered_bottles::find_by_bottle_id(&global, bottle_id)
        .map_err(|e| match e.downcast::<PillboxError>() {
            Ok(PillboxError::AmbiguousId {
                ref id_prefix,
                ref candidates,
            }) => err_409_ambiguous_id(id_prefix, candidates),
            Ok(PillboxError::InvalidId { ref id }) => err_400_invalid_id(id),
            Ok(other) => err_500(other.into()),
            Err(e) => err_500(e),
        })?
        .ok_or_else(|| err_404_bottle(bottle_id))?;
    db::connection::open_existing(std::path::Path::new(&reg.db_path)).map_err(err_500)
}

/// Abre una conexión a la DB global del sistema.
///
/// # Errors
///
/// Retorna `err_500` si la apertura de la conexión falla.
pub(super) fn open_global_conn(state: &AppState) -> Result<rusqlite::Connection, ApiResponse> {
    db::connection::open(&state.global_db_path).map_err(err_500)
}

/// Valor por defecto 30 para límites de paginación.
pub(super) fn default_30() -> u32 {
    30
}
/// Valor por defecto 50 para límites de paginación.
pub(super) fn default_50() -> u32 {
    50
}
