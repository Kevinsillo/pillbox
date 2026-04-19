use axum::{
    extract::{Path, State},
    Json,
};
use pillbox::{db::store, domain::prescription::NewPrescription, error::PillboxError};
use serde_json::json;
use validator::Validate;

use super::{err_409, err_404, err_422, err_500, ok, ok_created, open_conn, ApiResponse, AppState};

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
        Err(e) => match e.downcast::<PillboxError>() {
            Ok(PillboxError::PrescriptionAlreadyOpen {
                ref id,
                ref title,
                ref started_at,
                pill_count,
            }) => err_409(
                "prescription_already_open",
                &format!(
                    "prescription_already_open: '{title}' (id={id}, iniciada={started_at}, {pill_count} pills)"
                ),
                json!({
                    "id": id,
                    "title": title,
                    "started_at": started_at,
                    "pill_count": pill_count,
                }),
            ),
            Ok(other) => err_500(other.into()),
            Err(e) => err_500(e),
        },
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

pub async fn prescription_delete(
    State(s): State<AppState>,
    Path(id): Path<String>,
) -> ApiResponse {
    let mut conn = match open_conn(&s) {
        Ok(c) => c,
        Err(r) => return r,
    };
    match store::prescriptions::discard(&mut conn, &id) {
        Ok(()) => ok(json!({ "discarded": true })),
        Err(e) => err_500(e),
    }
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
