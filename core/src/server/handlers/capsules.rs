use axum::{
    extract::{Path, Query, State},
    Json,
};
use pillbox::{
    db::store,
    domain::capsule::{CapsulePatch, NewCapsule},
};
use serde::Deserialize;
use validator::Validate;

use super::{err_404_capsule, err_422, err_500, ok, ok_created, open_global_conn, ApiResponse, AppState};

#[derive(Deserialize)]
pub struct CapsuleListParams {
    pub compound: Option<String>,
    pub limit: Option<u32>,
}

#[derive(Deserialize)]
pub struct CapsuleSearchParams {
    pub query: String,
    pub compound: Option<String>,
    pub limit: Option<u32>,
}

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

pub async fn capsule_get(State(s): State<AppState>, Path(id): Path<i64>) -> ApiResponse {
    let conn = match open_global_conn(&s) {
        Ok(c) => c,
        Err(r) => return r,
    };
    match store::capsules::read(&conn, id) {
        Ok(Some(c)) => ok(c),
        Ok(None) => err_404_capsule(id),
        Err(e) => err_500(e),
    }
}

pub async fn capsule_patch(
    State(s): State<AppState>,
    Path(id): Path<i64>,
    Json(patch): Json<CapsulePatch>,
) -> ApiResponse {
    if let Err(e) = patch.validate() {
        return err_422(e);
    }
    let mut conn = match open_global_conn(&s) {
        Ok(c) => c,
        Err(r) => return r,
    };
    match store::capsules::revise(&mut conn, id, &patch) {
        Ok(Some(c)) => ok(c),
        Ok(None) => err_404_capsule(id),
        Err(e) => err_500(e),
    }
}

pub async fn capsule_delete(State(s): State<AppState>, Path(id): Path<i64>) -> ApiResponse {
    let mut conn = match open_global_conn(&s) {
        Ok(c) => c,
        Err(r) => return r,
    };
    match store::capsules::discard(&mut conn, id) {
        Ok(Some(r)) => ok(r),
        Ok(None) => err_404_capsule(id),
        Err(e) => err_500(e),
    }
}

pub async fn capsule_search(
    State(s): State<AppState>,
    Query(params): Query<CapsuleSearchParams>,
) -> ApiResponse {
    let conn = match open_global_conn(&s) {
        Ok(c) => c,
        Err(r) => return r,
    };
    match store::search::capsule_find(&conn, &params.query, params.compound.as_deref(), params.limit) {
        Ok(results) => ok(results),
        Err(e) => err_500(e),
    }
}

pub async fn capsule_list(
    State(s): State<AppState>,
    Query(params): Query<CapsuleListParams>,
) -> ApiResponse {
    let conn = match open_global_conn(&s) {
        Ok(c) => c,
        Err(r) => return r,
    };
    match store::capsules::list(&conn, params.limit, params.compound.as_deref()) {
        Ok(capsules) => ok(capsules),
        Err(e) => err_500(e),
    }
}
