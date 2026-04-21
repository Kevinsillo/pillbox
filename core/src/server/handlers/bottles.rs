use axum::{
    extract::{Path, Query, State},
    Json,
};
use pillbox::{
    db::{self, store, store::registered_bottles},
    domain::bottle::NewBottle,
};
use serde::Deserialize;
use validator::Validate;

use super::{
    default_50, err_404_bottle, err_422, err_500, ok, ok_created, open_conn, open_global_conn,
    ApiResponse, AppState,
};

#[derive(Deserialize)]
pub struct BottlePrescriptionsParams {
    #[serde(default = "default_50")]
    pub limit: u32,
}

pub async fn bottle_get(State(s): State<AppState>, Path(id): Path<i64>) -> ApiResponse {
    let conn = match open_conn(&s) {
        Ok(c) => c,
        Err(r) => return r,
    };
    match store::bottles::find_by_id(&conn, id) {
        Ok(Some(b)) => ok(b),
        Ok(None) => err_404_bottle(id),
        Err(e) => err_500(e),
    }
}

pub async fn bottle_list(State(s): State<AppState>) -> ApiResponse {
    let mut all_bottles: Vec<pillbox::domain::bottle::Bottle> = {
        let conn = match open_conn(&s) {
            Ok(c) => c,
            Err(r) => return r,
        };
        match store::bottles::list(&conn) {
            Ok(b) => b,
            Err(e) => return err_500(e),
        }
    };

    if s.db_path != s.global_db_path {
        let registered = {
            let global_conn = match open_global_conn(&s) {
                Ok(c) => c,
                Err(_) => return ok(all_bottles),
            };
            match registered_bottles::list(&global_conn) {
                Ok(r) => r,
                Err(_) => vec![],
            }
        };

        let mut seen_dirs: std::collections::HashSet<String> =
            all_bottles.iter().map(|b| b.directory.clone()).collect();

        for reg in registered {
            let reg_db_path = std::path::Path::new(&reg.db_path);
            if !reg_db_path.exists() {
                continue;
            }
            if reg_db_path == s.db_path.as_ref() {
                continue;
            }
            let remote_conn = match db::connection::open(reg_db_path) {
                Ok(c) => c,
                Err(_) => continue,
            };
            let remote_bottles = match store::bottles::list(&remote_conn) {
                Ok(b) => b,
                Err(_) => continue,
            };
            for bottle in remote_bottles {
                if seen_dirs.insert(bottle.directory.clone()) {
                    all_bottles.push(bottle);
                }
            }
        }
    }

    all_bottles.sort_by(|a, b| a.display_name.cmp(&b.display_name));
    ok(all_bottles)
}

pub async fn bottle_create(State(s): State<AppState>, Json(input): Json<NewBottle>) -> ApiResponse {
    if let Err(e) = input.validate() {
        return err_422(e);
    }
    let mut conn = match open_conn(&s) {
        Ok(c) => c,
        Err(r) => return r,
    };
    match store::bottles::create(&mut conn, &input) {
        Ok(b) => ok_created(b),
        Err(e) => err_500(e),
    }
}

pub async fn bottle_prescriptions(
    State(s): State<AppState>,
    Path(id): Path<i64>,
    Query(params): Query<BottlePrescriptionsParams>,
) -> ApiResponse {
    let conn = match open_conn(&s) {
        Ok(c) => c,
        Err(r) => return r,
    };
    match store::bottles::find_by_id(&conn, id) {
        Ok(None) => return err_404_bottle(id),
        Err(e) => return err_500(e),
        Ok(Some(_)) => {}
    }
    match store::prescriptions::list_by_bottle(&conn, id, params.limit) {
        Ok(rxs) => ok(rxs),
        Err(e) => err_500(e),
    }
}
