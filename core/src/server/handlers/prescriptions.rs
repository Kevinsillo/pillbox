use axum::{
    extract::{Path, State},
    Json,
};
use pillbox::{db::store, domain::prescription::NewPrescription, error::PillboxError};
use serde::Deserialize;
use serde_json::json;
use validator::Validate;

use super::{
    conn_for_bottle, err_404_prescription, err_409, err_422, err_500, ok, ok_created, ApiResponse,
    AppState,
};

#[derive(Deserialize, Validate)]
pub struct NewPrescriptionBody {
    #[validate(length(min = 1, max = 255))]
    pub title: String,
}

pub async fn prescription_open(
    State(s): State<AppState>,
    Path(bottle_id): Path<String>,
    Json(input): Json<NewPrescriptionBody>,
) -> ApiResponse {
    if let Err(e) = input.validate() {
        return err_422(e);
    }
    let mut conn = match conn_for_bottle(&s, &bottle_id) {
        Ok(c) => c,
        Err(r) => return r,
    };
    let new_rx = NewPrescription {
        bottle_id,
        title: input.title,
    };
    match store::prescriptions::open(&mut conn, &new_rx) {
        Ok(rx) => ok_created(rx),
        Err(e) => match e.downcast::<PillboxError>() {
            Ok(PillboxError::PrescriptionAlreadyOpen {
                ref id,
                ref title,
                ref started_at,
                pill_count,
            }) => err_409(
                "prescription_already_open",
                "prescription_already_open",
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

pub async fn prescription_get(
    State(s): State<AppState>,
    Path((bottle_id, rx_id)): Path<(String, String)>,
) -> ApiResponse {
    let conn = match conn_for_bottle(&s, &bottle_id) {
        Ok(c) => c,
        Err(r) => return r,
    };
    match store::prescriptions::read_any(&conn, &rx_id) {
        Ok(Some(rx)) => ok(rx),
        Ok(None) => err_404_prescription(&rx_id),
        Err(e) => err_500(e),
    }
}

pub async fn prescription_close(
    State(s): State<AppState>,
    Path((bottle_id, rx_id)): Path<(String, String)>,
) -> ApiResponse {
    let mut conn = match conn_for_bottle(&s, &bottle_id) {
        Ok(c) => c,
        Err(r) => return r,
    };
    match store::prescriptions::close(&mut conn, &rx_id) {
        Ok(rx) => ok(rx),
        Err(e) => err_500(e),
    }
}

pub async fn prescription_delete(
    State(s): State<AppState>,
    Path((bottle_id, rx_id)): Path<(String, String)>,
) -> ApiResponse {
    let mut conn = match conn_for_bottle(&s, &bottle_id) {
        Ok(c) => c,
        Err(r) => return r,
    };
    match store::prescriptions::discard(&mut conn, &rx_id) {
        Ok(()) => ok(json!({ "discarded": true })),
        Err(e) => err_500(e),
    }
}

pub async fn prescription_purge(
    State(s): State<AppState>,
    Path((bottle_id, rx_id)): Path<(String, String)>,
) -> ApiResponse {
    let mut conn = match conn_for_bottle(&s, &bottle_id) {
        Ok(c) => c,
        Err(r) => return r,
    };
    match store::prescriptions::hard_delete(&mut conn, &rx_id) {
        Ok(()) => ok(json!({ "purged": true })),
        Err(e) => match e.downcast::<PillboxError>() {
            Ok(PillboxError::PrescriptionNotFound { .. }) => err_404_prescription(&rx_id),
            Ok(other) => err_500(other.into()),
            Err(e) => err_500(e),
        },
    }
}

pub async fn prescription_pills(
    State(s): State<AppState>,
    Path((bottle_id, rx_id)): Path<(String, String)>,
) -> ApiResponse {
    let conn = match conn_for_bottle(&s, &bottle_id) {
        Ok(c) => c,
        Err(r) => return r,
    };
    match store::prescriptions::read_any(&conn, &rx_id) {
        Ok(None) => return err_404_prescription(&rx_id),
        Err(e) => return err_500(e),
        Ok(Some(_)) => {}
    }
    match store::pills::list_by_prescription(&conn, &rx_id) {
        Ok(pills) => ok(pills),
        Err(e) => err_500(e),
    }
}
