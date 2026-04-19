//! Handlers de todas las rutas HTTP.
//!
//! Todas las respuestas siguen el esquema:
//!   `{ "ok": true,  "data": { ... } }`          — éxito
//!   `{ "ok": false, "error": "...", "message": "..." }` — error

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use validator::Validate;

use pillbox::{
    db::{
        self,
        store::{self, registered_bottles, PrescriptionAlreadyOpen},
    },
    domain::{
        bottle::NewBottle,
        capsule::{CapsulePatch, NewCapsule},
        pill::{NewPill, PillPatch},
        prescription::NewPrescription,
        search::SearchParams,
    },
};

use super::AppState;

// ─── Tipo de respuesta uniforme ───────────────────────────────────────────────

pub(super) struct ApiResponse(StatusCode, Value);

impl IntoResponse for ApiResponse {
    fn into_response(self) -> Response {
        (self.0, Json(self.1)).into_response()
    }
}

fn ok(data: impl Serialize) -> ApiResponse {
    ApiResponse(StatusCode::OK, json!({ "ok": true, "data": data }))
}

fn ok_created(data: impl Serialize) -> ApiResponse {
    ApiResponse(StatusCode::CREATED, json!({ "ok": true, "data": data }))
}

pub(super) async fn version_get() -> ApiResponse {
    ok(json!({ "version": env!("CARGO_PKG_VERSION") }))
}

fn err(status: StatusCode, error: &str, message: &str) -> ApiResponse {
    ApiResponse(
        status,
        json!({ "ok": false, "error": error, "message": message }),
    )
}

fn err_422(e: impl ToString) -> ApiResponse {
    err(
        StatusCode::UNPROCESSABLE_ENTITY,
        "validation_error",
        &e.to_string(),
    )
}

fn err_404(what: &str, id: impl std::fmt::Display) -> ApiResponse {
    err(
        StatusCode::NOT_FOUND,
        "not_found",
        &format!("{} {} no encontrado", what, id),
    )
}

fn err_500(e: anyhow::Error) -> ApiResponse {
    err(StatusCode::INTERNAL_SERVER_ERROR, "error", &e.to_string())
}

fn err_409(error: &str, message: &str, data: Value) -> ApiResponse {
    ApiResponse(
        StatusCode::CONFLICT,
        json!({ "ok": false, "error": error, "message": message, "data": data }),
    )
}

fn open_conn(state: &AppState) -> Result<rusqlite::Connection, ApiResponse> {
    db::connection::open(&state.db_path).map_err(err_500)
}

fn open_global_conn(state: &AppState) -> Result<rusqlite::Connection, ApiResponse> {
    db::connection::open(&state.global_db_path).map_err(err_500)
}

// ─── Pills ────────────────────────────────────────────────────────────────────

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

// ─── Capsules ─────────────────────────────────────────────────────────────────

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
        Ok(None) => err_404("capsule", id),
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
        Ok(None) => err_404("capsule", id),
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
        Ok(None) => err_404("capsule", id),
        Err(e) => err_500(e),
    }
}

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

// ─── Prescriptions ────────────────────────────────────────────────────────────

pub async fn prescription_open(
    State(s): State<AppState>,
    Json(input): Json<NewPrescription>,
) -> ApiResponse {
    if let Err(e) = input.validate() {
        return err_422(e);
    }
    let mut conn = match open_conn(&s) {
        Ok(c) => c,
        Err(r) => return r,
    };
    match store::prescriptions::open(&mut conn, &input) {
        Ok(rx) => ok_created(rx),
        Err(e) => {
            if let Some(already) = e.downcast_ref::<PrescriptionAlreadyOpen>() {
                err_409(
                    "prescription_already_open",
                    &already.to_string(),
                    serde_json::to_value(already).unwrap_or(Value::Null),
                )
            } else {
                err_500(e)
            }
        }
    }
}

pub async fn prescription_get(State(s): State<AppState>, Path(id): Path<String>) -> ApiResponse {
    let conn = match open_conn(&s) {
        Ok(c) => c,
        Err(r) => return r,
    };
    match store::prescriptions::read(&conn, &id) {
        Ok(Some(rx)) => ok(rx),
        Ok(None) => err_404("prescription", &id),
        Err(e) => err_500(e),
    }
}

pub async fn prescription_close(State(s): State<AppState>, Path(id): Path<String>) -> ApiResponse {
    let mut conn = match open_conn(&s) {
        Ok(c) => c,
        Err(r) => return r,
    };
    match store::prescriptions::close(&mut conn, &id) {
        Ok(rx) => ok(rx),
        Err(e) => err_500(e),
    }
}

pub async fn prescription_delete(State(s): State<AppState>, Path(id): Path<String>) -> ApiResponse {
    let mut conn = match open_conn(&s) {
        Ok(c) => c,
        Err(r) => return r,
    };
    match store::prescriptions::discard(&mut conn, &id) {
        Ok(()) => ok(json!({ "discarded": true })),
        Err(e) => err_500(e),
    }
}

// ─── Bottles ──────────────────────────────────────────────────────────────────

pub async fn bottle_get(State(s): State<AppState>, Path(id): Path<i64>) -> ApiResponse {
    let conn = match open_conn(&s) {
        Ok(c) => c,
        Err(r) => return r,
    };
    match store::bottles::find_by_id(&conn, id) {
        Ok(Some(b)) => ok(b),
        Ok(None) => err_404("bottle", id),
        Err(e) => err_500(e),
    }
}

pub async fn bottle_list(State(s): State<AppState>) -> ApiResponse {
    // 1. Bottles de la DB activa (local o global)
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

    // 2. Bottles de otras DBs locales registradas en la global
    //    Solo cuando db_path != global_db_path (evitar doble lectura)
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
                continue; // proyecto movido o borrado
            }
            if reg_db_path == s.db_path.as_ref() {
                continue; // ya leído en paso 1
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
        Ok(None) => return err_404("bottle", id),
        Err(e) => return err_500(e),
        Ok(Some(_)) => {}
    }
    match store::prescriptions::list_by_bottle(&conn, id, params.limit) {
        Ok(rxs) => ok(rxs),
        Err(e) => err_500(e),
    }
}

#[derive(Deserialize)]
pub struct BottlePrescriptionsParams {
    #[serde(default = "default_50")]
    pub limit: u32,
}

pub async fn prescription_pills(State(s): State<AppState>, Path(id): Path<String>) -> ApiResponse {
    let conn = match open_conn(&s) {
        Ok(c) => c,
        Err(r) => return r,
    };
    match store::prescriptions::read(&conn, &id) {
        Ok(None) => return err_404("prescription", &id),
        Err(e) => return err_500(e),
        Ok(Some(_)) => {}
    }
    match store::pills::list_by_prescription(&conn, &id) {
        Ok(pills) => ok(pills),
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

// ─── Context ──────────────────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct ContextParams {
    pub bottle_id: i64,
    #[serde(default = "default_5")]
    pub prescription_limit: u32,
    #[serde(default = "default_30")]
    pub pill_limit: u32,
}

pub async fn context_get(
    State(s): State<AppState>,
    Query(params): Query<ContextParams>,
) -> ApiResponse {
    let conn = match open_conn(&s) {
        Ok(c) => c,
        Err(r) => return r,
    };
    let ctx = match store::search::pill_context(
        &conn,
        params.bottle_id,
        params.prescription_limit,
        params.pill_limit,
    ) {
        Ok(c) => c,
        Err(e) => return err_500(e),
    };
    let pills = match store::search::recent_pills(&conn, params.bottle_id, params.pill_limit) {
        Ok(p) => p,
        Err(e) => return err_500(e),
    };
    ok(json!({
        "context":            pills,
        "prescription_count": ctx.prescription_count,
        "pill_count":         ctx.pill_count,
    }))
}

fn default_5() -> u32 {
    5
}
fn default_30() -> u32 {
    30
}
fn default_50() -> u32 {
    50
}
