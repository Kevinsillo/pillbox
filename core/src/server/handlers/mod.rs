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

use std::path::PathBuf;

use r2d2::{Pool, PooledConnection};
use r2d2_sqlite::SqliteConnectionManager;

use crate::db::connection::build_pool;
use crate::db::store::registered_bottles;
use crate::db::DbScope;
use crate::error::PillboxError;

use super::AppState;

/// Envuelve un bloque síncrono de DB en `spawn_blocking` y mapea el
/// `JoinError` a `err_500`. Todos los handlers que tocan rusqlite deben
/// ejecutarse a través de este helper para no bloquear el runtime axum
/// (queries síncronas + lock de WAL + busy_timeout=5s pueden congelar
/// workers async durante segundos — ver cutover Fase 2, riesgo r2).
pub(super) async fn blocking<F>(f: F) -> ApiResponse
where
    F: FnOnce() -> ApiResponse + Send + 'static,
{
    match tokio::task::spawn_blocking(f).await {
        Ok(r) => r,
        Err(e) => err_500(anyhow::anyhow!("join error: {}", e)),
    }
}

/// Alias del tipo de conexión que entregan los pools al resto de handlers.
///
/// Implementa `Deref<Target = rusqlite::Connection>`, así que las firmas que
/// antes recibían `&Connection` siguen funcionando. Para callers que necesitan
/// `&mut Connection` (e.g. `Connection::transaction`), `PooledConnection` también
/// expone `DerefMut`.
pub(super) type PooledConn = PooledConnection<SqliteConnectionManager>;

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

/// Handler `GET /api/info` — devuelve versión del binario y datos del entorno (os/arch/family).
pub(super) async fn info_get() -> ApiResponse {
    ok(json!({
        "version": env!("CARGO_PKG_VERSION"),
        "os": std::env::consts::OS,
        "arch": std::env::consts::ARCH,
        "family": std::env::consts::FAMILY,
    }))
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
        json!({ "id": id, "message": "ID must be at least 8 characters long" }),
    )
}

/// Respuesta 400 Bad Request por parámetros de paginación fuera de rango.
pub(super) fn err_400_pagination(msg: &str) -> ApiResponse {
    err(StatusCode::BAD_REQUEST, "invalid_pagination", msg)
}

/// Respuesta 409 para un prefijo de ID ambiguo (coincide con >1 registro).
pub(super) fn err_409_ambiguous_id(prefix: &str, candidates: &[String]) -> ApiResponse {
    err_with_context(
        StatusCode::CONFLICT,
        "ambiguous_id",
        json!({ "id_prefix": prefix, "candidates": candidates }),
    )
}

/// Devuelve (o construye) el pool r2d2 asociado al `path` de un bottle local.
///
/// Mantiene un LRU compartido `state.bottle_pools` (Mutex<LruCache>) con cap
/// `MAX_BOTTLE_POOLS`. Crea un nuevo pool la primera vez que se ve cada path;
/// cada `get` promociona la entrada al frente del LRU. Cuando se inserta uno
/// nuevo y el cap está lleno, el menos recientemente usado se Dropea (r2d2
/// cierra sus conexiones libres; las en uso siguen vivas por el Arc interno).
pub(super) fn bottle_pool_for(
    s: &AppState,
    path: &std::path::Path,
) -> Result<Pool<SqliteConnectionManager>, ApiResponse> {
    let mut cache = s
        .bottle_pools
        .lock()
        .map_err(|e| err_500(anyhow::anyhow!("bottle_pools mutex poisoned: {}", e)))?;
    if let Some(p) = cache.get(path) {
        return Ok(p.clone());
    }
    let pool = build_pool(path, DbScope::Local).map_err(err_500)?;
    cache.put(path.to_path_buf(), pool.clone());
    Ok(pool)
}

/// Abre la DB local del bottle identificado por UUID.
/// Busca la ruta en `registered_bottles` (DB global) y obtiene/crea su pool.
pub(super) fn conn_for_bottle(s: &AppState, bottle_id: &str) -> Result<PooledConn, ApiResponse> {
    let global = open_global_conn(s)?;
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
    drop(global);
    let path = PathBuf::from(&reg.db_path);
    let pool = bottle_pool_for(s, &path)?;
    pool.get().map_err(|e| err_500(anyhow::anyhow!(e)))
}

/// Abre la DB del bottle y devuelve la conexión junto con el UUID completo del bottle.
///
/// Necesario cuando el caller debe pasar el bottle_id como foreign key a otras queries
/// (e.g. list_by_bottle, count_by_bottle) que requieren el UUID completo, no un prefijo.
pub(super) fn conn_for_bottle_with_id(
    s: &AppState,
    bottle_id: &str,
) -> Result<(PooledConn, String), ApiResponse> {
    use crate::db::store;
    let conn = conn_for_bottle(s, bottle_id)?;
    match store::bottles::find_by_id(&conn, bottle_id) {
        Ok(Some(b)) => Ok((conn, b.id)),
        Ok(None) => Err(err_404_bottle(bottle_id)),
        Err(e) => Err(err_500(e)),
    }
}

/// Devuelve una conexión a la DB global del pool eager construido en startup.
///
/// # Errors
///
/// Retorna `err_500` si el pool no puede entregar conexión en `busy_timeout`.
pub(super) fn open_global_conn(state: &AppState) -> Result<PooledConn, ApiResponse> {
    state
        .global_pool
        .get()
        .map_err(|e| err_500(anyhow::anyhow!(e)))
}

/// Valor por defecto 30 para `BottleStatsParams.days`.
pub(super) fn default_30() -> u32 {
    30
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn info_get_returns_all_fields() {
        let resp = info_get().await;
        let body = resp.1;
        let data = body.get("data").expect("data field present");
        for key in ["version", "os", "arch", "family"] {
            let v = data.get(key).unwrap_or_else(|| panic!("missing {key}"));
            let s = v.as_str().unwrap_or_else(|| panic!("{key} not a string"));
            assert!(!s.is_empty(), "{key} is empty");
        }
    }
}
