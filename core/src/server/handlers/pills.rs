use axum::{
    extract::{Path, Query, State},
    Json,
};
use pillbox::{
    db::{self, store, store::registered_bottles},
    domain::{
        pill::{NewPill, PillCompound, PillPatch},
        search::SearchParams,
    },
};
use serde::Deserialize;
use serde_json::json;
use validator::Validate;

use super::{
    conn_for_bottle, err_404_pill, err_422, err_500, ok, ok_created, open_global_conn, ApiResponse,
    AppState,
};

#[derive(Deserialize, Validate)]
pub struct NewPillBody {
    #[validate(length(min = 1, max = 255))]
    pub title: String,
    #[validate(length(min = 1, max = 5000))]
    pub content: String,
    pub compound: PillCompound,
    pub author_name: Option<String>,
    pub author_email: Option<String>,
}

pub async fn pill_create(
    State(s): State<AppState>,
    Path((bottle_id, rx_id)): Path<(String, String)>,
    Json(input): Json<NewPillBody>,
) -> ApiResponse {
    if let Err(e) = input.validate() {
        return err_422(e);
    }
    let mut conn = match conn_for_bottle(&s, &bottle_id) {
        Ok(c) => c,
        Err(r) => return r,
    };
    let new_pill = NewPill {
        title: input.title,
        content: input.content,
        compound: input.compound,
        prescription_id: rx_id,
        author_name: input.author_name,
        author_email: input.author_email,
    };
    match store::pills::take(&mut conn, &new_pill) {
        Ok(r) => ok_created(r),
        Err(e) => err_500(e),
    }
}

pub async fn pill_get(
    State(s): State<AppState>,
    Path((bottle_id, _rx_id, pill_id)): Path<(String, String, i64)>,
) -> ApiResponse {
    let conn = match conn_for_bottle(&s, &bottle_id) {
        Ok(c) => c,
        Err(r) => return r,
    };
    match store::pills::read_any(&conn, pill_id) {
        Ok(Some(p)) => ok(p),
        Ok(None) => err_404_pill(pill_id),
        Err(e) => err_500(e),
    }
}

pub async fn pill_patch(
    State(s): State<AppState>,
    Path((bottle_id, _rx_id, pill_id)): Path<(String, String, i64)>,
    Json(patch): Json<PillPatch>,
) -> ApiResponse {
    if let Err(e) = patch.validate() {
        return err_422(e);
    }
    let mut conn = match conn_for_bottle(&s, &bottle_id) {
        Ok(c) => c,
        Err(r) => return r,
    };
    match store::pills::revise(&mut conn, pill_id, &patch) {
        Ok(Some(p)) => ok(p),
        Ok(None) => err_404_pill(pill_id),
        Err(e) => err_500(e),
    }
}

pub async fn pill_delete(
    State(s): State<AppState>,
    Path((bottle_id, _rx_id, pill_id)): Path<(String, String, i64)>,
) -> ApiResponse {
    let mut conn = match conn_for_bottle(&s, &bottle_id) {
        Ok(c) => c,
        Err(r) => return r,
    };
    match store::pills::discard(&mut conn, pill_id) {
        Ok(Some(r)) => ok(r),
        Ok(None) => err_404_pill(pill_id),
        Err(e) => err_500(e),
    }
}

pub async fn pill_purge(
    State(s): State<AppState>,
    Path((bottle_id, _rx_id, pill_id)): Path<(String, String, i64)>,
) -> ApiResponse {
    let mut conn = match conn_for_bottle(&s, &bottle_id) {
        Ok(c) => c,
        Err(r) => return r,
    };
    match store::pills::hard_delete(&mut conn, pill_id) {
        Ok(Some(_)) => ok(json!({ "purged": true })),
        Ok(None) => err_404_pill(pill_id),
        Err(e) => err_500(e),
    }
}

pub async fn pill_search(
    State(s): State<AppState>,
    Query(params): Query<SearchParams>,
) -> ApiResponse {
    if let Some(ref bottle_id) = params.bottle_id {
        let conn = match conn_for_bottle(&s, bottle_id) {
            Ok(c) => c,
            Err(r) => return r,
        };
        match store::search::pill_find(&conn, &params) {
            Ok(results) => ok(results),
            Err(e) => err_500(e),
        }
    } else {
        // Sin bottle_id: agrega resultados de todas las DBs registradas
        let global_conn = match open_global_conn(&s) {
            Ok(c) => c,
            Err(r) => return r,
        };
        let registered = match registered_bottles::list(&global_conn).map_err(err_500) {
            Ok(r) => r,
            Err(r) => return r,
        };
        let mut all_results = vec![];
        for reg in &registered {
            let path = std::path::Path::new(&reg.db_path);
            if let Ok(conn) = db::connection::open_existing(path) {
                if let Ok(mut results) = store::search::pill_find(&conn, &params) {
                    all_results.append(&mut results);
                }
            }
        }
        all_results.sort_by(|a, b| {
            a.rank
                .partial_cmp(&b.rank)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        ok(all_results)
    }
}
