use axum::{
    extract::{Path, Query, State},
    Json,
};
use pillbox::{
    db::store,
    domain::{
        pill::{NewPill, PillPatch},
        search::SearchParams,
    },
};
use validator::Validate;

use super::{err_404, err_422, err_500, ok, ok_created, open_conn, ApiResponse, AppState};

pub async fn pill_create(State(s): State<AppState>, Json(input): Json<NewPill>) -> ApiResponse {
    if let Err(e) = input.validate() {
        return err_422(e);
    }
    let mut conn = match open_conn(&s) {
        Ok(c) => c,
        Err(r) => return r,
    };
    match store::pills::take(&mut conn, &input) {
        Ok(r) => ok_created(r),
        Err(e) => err_500(e),
    }
}

pub async fn pill_get(State(s): State<AppState>, Path(id): Path<i64>) -> ApiResponse {
    let conn = match open_conn(&s) {
        Ok(c) => c,
        Err(r) => return r,
    };
    match store::pills::read(&conn, id) {
        Ok(Some(p)) => ok(p),
        Ok(None) => err_404("pill", id),
        Err(e) => err_500(e),
    }
}

pub async fn pill_patch(
    State(s): State<AppState>,
    Path(id): Path<i64>,
    Json(patch): Json<PillPatch>,
) -> ApiResponse {
    if let Err(e) = patch.validate() {
        return err_422(e);
    }
    let mut conn = match open_conn(&s) {
        Ok(c) => c,
        Err(r) => return r,
    };
    match store::pills::revise(&mut conn, id, &patch) {
        Ok(Some(p)) => ok(p),
        Ok(None) => err_404("pill", id),
        Err(e) => err_500(e),
    }
}

pub async fn pill_delete(State(s): State<AppState>, Path(id): Path<i64>) -> ApiResponse {
    let mut conn = match open_conn(&s) {
        Ok(c) => c,
        Err(r) => return r,
    };
    match store::pills::discard(&mut conn, id) {
        Ok(Some(r)) => ok(r),
        Ok(None) => err_404("pill", id),
        Err(e) => err_500(e),
    }
}

pub async fn pill_search(
    State(s): State<AppState>,
    Query(params): Query<SearchParams>,
) -> ApiResponse {
    let conn = match open_conn(&s) {
        Ok(c) => c,
        Err(r) => return r,
    };
    match store::search::pill_find(&conn, &params) {
        Ok(results) => ok(results),
        Err(e) => err_500(e),
    }
}
