//! Handlers HTTP para la entidad Capsule (conocimiento personal global).

use axum::{
    extract::{Path, Query, State},
    Json,
};
use pillbox::{
    db::{store, store::ListFilter},
    domain::capsule::{CapsulePatch, NewCapsule},
    error::PillboxError,
};
use serde::Deserialize;
use validator::Validate;

use super::{
    err_400_invalid_id, err_404_capsule, err_409_ambiguous_id, err_422, err_500, ok, ok_created,
    open_global_conn, ApiResponse, AppState,
};

/// Parámetros de query para listar capsules con filtro opcional por compound.
#[derive(Deserialize)]
pub struct CapsuleListParams {
    pub compound: Option<String>,
    pub limit: Option<u32>,
}

/// Parámetros de query para la búsqueda FTS5 de capsules.
#[derive(Deserialize)]
pub struct CapsuleSearchParams {
    pub query: String,
    pub compound: Option<String>,
    pub limit: Option<u32>,
}

/// Handler `POST /api/capsules` — crea una capsule nueva en la DB global.
pub async fn capsule_create(
    State(s): State<AppState>,
    Json(input): Json<NewCapsule>,
) -> ApiResponse {
    if let Err(e) = input.validate() {
        return err_422(e);
    }
    let mut conn = match open_global_conn(&s) {
        Ok(c) => c,
        Err(r) => return r,
    };
    match store::capsules::take(&mut conn, &input) {
        Ok(r) => ok_created(r),
        Err(e) => err_500(e),
    }
}

/// Handler `GET /api/capsules/:id` — lee una capsule (incluyendo archivadas) por UUID.
pub async fn capsule_get(State(s): State<AppState>, Path(id): Path<String>) -> ApiResponse {
    let conn = match open_global_conn(&s) {
        Ok(c) => c,
        Err(r) => return r,
    };
    match store::capsules::read_any(&conn, &id) {
        Ok(Some(c)) => ok(c),
        Ok(None) => err_404_capsule(id),
        Err(e) => match e.downcast::<PillboxError>() {
            Ok(PillboxError::AmbiguousId {
                ref id_prefix,
                ref candidates,
            }) => err_409_ambiguous_id(id_prefix, candidates),
            Ok(PillboxError::InvalidId { ref id }) => err_400_invalid_id(id),
            Ok(other) => err_500(other.into()),
            Err(e) => err_500(e),
        },
    }
}

/// Handler `DELETE /api/capsules/:id/purge` — elimina permanentemente una capsule.
pub async fn capsule_purge(State(s): State<AppState>, Path(id): Path<String>) -> ApiResponse {
    use serde_json::json;
    let mut conn = match open_global_conn(&s) {
        Ok(c) => c,
        Err(r) => return r,
    };
    match store::capsules::hard_delete(&mut conn, &id) {
        Ok(true) => ok(json!({ "purged": true })),
        Ok(false) => err_404_capsule(id),
        Err(e) => match e.downcast::<PillboxError>() {
            Ok(PillboxError::AmbiguousId {
                ref id_prefix,
                ref candidates,
            }) => err_409_ambiguous_id(id_prefix, candidates),
            Ok(PillboxError::InvalidId { ref id }) => err_400_invalid_id(id),
            Ok(other) => err_500(other.into()),
            Err(e) => err_500(e),
        },
    }
}

/// Handler `PATCH /api/capsules/:id` — aplica un patch parcial sobre una capsule.
pub async fn capsule_patch(
    State(s): State<AppState>,
    Path(id): Path<String>,
    Json(patch): Json<CapsulePatch>,
) -> ApiResponse {
    if let Err(e) = patch.validate() {
        return err_422(e);
    }
    let mut conn = match open_global_conn(&s) {
        Ok(c) => c,
        Err(r) => return r,
    };
    match store::capsules::revise(&mut conn, &id, &patch) {
        Ok(Some(c)) => ok(c),
        Ok(None) => err_404_capsule(id),
        Err(e) => match e.downcast::<PillboxError>() {
            Ok(PillboxError::AmbiguousId {
                ref id_prefix,
                ref candidates,
            }) => err_409_ambiguous_id(id_prefix, candidates),
            Ok(PillboxError::InvalidId { ref id }) => err_400_invalid_id(id),
            Ok(other) => err_500(other.into()),
            Err(e) => err_500(e),
        },
    }
}

/// Handler `DELETE /api/capsules/:id` — realiza un soft delete de una capsule.
pub async fn capsule_delete(State(s): State<AppState>, Path(id): Path<String>) -> ApiResponse {
    let mut conn = match open_global_conn(&s) {
        Ok(c) => c,
        Err(r) => return r,
    };
    match store::capsules::discard(&mut conn, &id) {
        Ok(Some(r)) => ok(r),
        Ok(None) => err_404_capsule(id),
        Err(e) => match e.downcast::<PillboxError>() {
            Ok(PillboxError::AmbiguousId {
                ref id_prefix,
                ref candidates,
            }) => err_409_ambiguous_id(id_prefix, candidates),
            Ok(PillboxError::InvalidId { ref id }) => err_400_invalid_id(id),
            Ok(other) => err_500(other.into()),
            Err(e) => err_500(e),
        },
    }
}

/// Handler `GET /api/capsules/search` — busca capsules con FTS5 y expansión fuzzy.
pub async fn capsule_search(
    State(s): State<AppState>,
    Query(params): Query<CapsuleSearchParams>,
) -> ApiResponse {
    let conn = match open_global_conn(&s) {
        Ok(c) => c,
        Err(r) => return r,
    };
    match store::search::capsule_find(
        &conn,
        &params.query,
        params.compound.as_deref(),
        params.limit,
    ) {
        Ok(results) => ok(results),
        Err(e) => err_500(e),
    }
}

/// Handler `GET /api/capsules` — lista capsules activas con filtro opcional por compound.
pub async fn capsule_list(
    State(s): State<AppState>,
    Query(params): Query<CapsuleListParams>,
) -> ApiResponse {
    let conn = match open_global_conn(&s) {
        Ok(c) => c,
        Err(r) => return r,
    };
    match store::capsules::list(&conn, params.limit, params.compound.as_deref(), ListFilter::All) {
        Ok(capsules) => ok(capsules),
        Err(e) => err_500(e),
    }
}
