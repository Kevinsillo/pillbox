//! Handlers HTTP para la entidad Prescription.
//!
//! Las queries rusqlite van envueltas en `spawn_blocking`.

use crate::{
    db::store,
    domain::{prescription::NewPrescription, PaginationParams},
    error::PillboxError,
};
use axum::{
    extract::{Path, Query, State},
    Json,
};
use serde::Deserialize;
use serde_json::json;
use validator::Validate;

use super::{
    blocking, conn_for_bottle, conn_for_bottle_with_id, err_400_invalid_id, err_400_pagination,
    err_404_prescription, err_409_ambiguous_id, err_422, err_500, ok, ok_created, ApiResponse,
    AppState,
};

/// Cuerpo JSON para abrir una prescripción nueva via REST API.
#[derive(Deserialize, Validate)]
pub struct NewPrescriptionBody {
    #[validate(length(min = 1, max = 255))]
    pub title: String,
}

/// Handler `POST /api/bottles/:id/prescriptions` — abre una prescripción nueva.
///
/// Múltiples prescriptions abiertas en el mismo bottle están permitidas.
pub async fn prescription_open(
    State(s): State<AppState>,
    Path(bottle_id): Path<String>,
    Json(input): Json<NewPrescriptionBody>,
) -> ApiResponse {
    if let Err(e) = input.validate() {
        return err_422(e);
    }
    blocking(move || {
        let (mut conn, full_bottle_id) = match conn_for_bottle_with_id(&s, &bottle_id) {
            Ok(v) => v,
            Err(r) => return r,
        };
        let new_rx = NewPrescription {
            bottle_id: full_bottle_id,
            title: input.title,
            author_name: None,
            author_email: None,
        };
        match store::prescriptions::open(&mut conn, &new_rx) {
            Ok(rx) => ok_created(rx),
            Err(e) => err_500(e),
        }
    })
    .await
}

/// Handler `GET /api/bottles/:id/prescriptions/:rx_id` — lee una prescripción por ID.
pub async fn prescription_get(
    State(s): State<AppState>,
    Path((bottle_id, rx_id)): Path<(String, String)>,
) -> ApiResponse {
    blocking(move || {
        let conn = match conn_for_bottle(&s, &bottle_id) {
            Ok(c) => c,
            Err(r) => return r,
        };
        match store::prescriptions::read_any(&conn, &rx_id) {
            Ok(Some(rx)) => ok(rx),
            Ok(None) => err_404_prescription(&rx_id),
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
    })
    .await
}

/// Handler `PATCH /api/bottles/:id/prescriptions/:rx_id` — cierra una prescripción activa.
pub async fn prescription_close(
    State(s): State<AppState>,
    Path((bottle_id, rx_id)): Path<(String, String)>,
) -> ApiResponse {
    blocking(move || {
        let mut conn = match conn_for_bottle(&s, &bottle_id) {
            Ok(c) => c,
            Err(r) => return r,
        };
        match store::prescriptions::close(&mut conn, &rx_id) {
            Ok(rx) => ok(rx),
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
    })
    .await
}

/// Handler `POST /api/bottles/:id/prescriptions/:rx_id/reopen` — reabre una prescription cerrada.
///
/// Idempotente: si la prescription ya está abierta, devuelve 200 OK con la rx
/// sin modificar. 404 si la prescription no existe (o está descartada).
pub async fn prescription_reopen(
    State(s): State<AppState>,
    Path((bottle_id, rx_id)): Path<(String, String)>,
) -> ApiResponse {
    blocking(move || {
        let mut conn = match conn_for_bottle(&s, &bottle_id) {
            Ok(c) => c,
            Err(r) => return r,
        };
        match store::prescriptions::reopen(&mut conn, &rx_id) {
            Ok(rx) => ok(rx),
            Err(e) => match e.downcast::<PillboxError>() {
                Ok(PillboxError::PrescriptionNotFound { .. }) => err_404_prescription(&rx_id),
                Ok(PillboxError::AmbiguousId {
                    ref id_prefix,
                    ref candidates,
                }) => err_409_ambiguous_id(id_prefix, candidates),
                Ok(PillboxError::InvalidId { ref id }) => err_400_invalid_id(id),
                Ok(other) => err_500(other.into()),
                Err(e) => err_500(e),
            },
        }
    })
    .await
}

/// Handler `DELETE /api/bottles/:id/prescriptions/:rx_id` — realiza un soft delete en cascada.
pub async fn prescription_delete(
    State(s): State<AppState>,
    Path((bottle_id, rx_id)): Path<(String, String)>,
) -> ApiResponse {
    blocking(move || {
        let mut conn = match conn_for_bottle(&s, &bottle_id) {
            Ok(c) => c,
            Err(r) => return r,
        };
        match store::prescriptions::discard(&mut conn, &rx_id) {
            Ok(()) => ok(json!({ "discarded": true })),
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
    })
    .await
}

/// Handler `DELETE /api/.../prescriptions/:rx_id/purge` — elimina permanentemente una prescripción.
pub async fn prescription_purge(
    State(s): State<AppState>,
    Path((bottle_id, rx_id)): Path<(String, String)>,
) -> ApiResponse {
    blocking(move || {
        let mut conn = match conn_for_bottle(&s, &bottle_id) {
            Ok(c) => c,
            Err(r) => return r,
        };
        match store::prescriptions::hard_delete(&mut conn, &rx_id) {
            Ok(()) => ok(json!({ "purged": true })),
            Err(e) => match e.downcast::<PillboxError>() {
                Ok(PillboxError::PrescriptionNotFound { .. }) => err_404_prescription(&rx_id),
                Ok(PillboxError::AmbiguousId {
                    ref id_prefix,
                    ref candidates,
                }) => err_409_ambiguous_id(id_prefix, candidates),
                Ok(PillboxError::InvalidId { ref id }) => err_400_invalid_id(id),
                Ok(other) => err_500(other.into()),
                Err(e) => err_500(e),
            },
        }
    })
    .await
}

/// Handler `GET /api/.../prescriptions/:rx_id/pills` — lista las pills de una prescripción.
///
/// Acepta `?page=&page_size=`.
pub async fn prescription_pills(
    State(s): State<AppState>,
    Path((bottle_id, rx_id)): Path<(String, String)>,
    Query(pagination): Query<PaginationParams>,
) -> ApiResponse {
    if let Err(e) = pagination.validate() {
        return err_400_pagination(&e);
    }
    blocking(move || {
        let conn = match conn_for_bottle(&s, &bottle_id) {
            Ok(c) => c,
            Err(r) => return r,
        };
        let rx = match store::prescriptions::read_any(&conn, &rx_id) {
            Ok(Some(rx)) => rx,
            Ok(None) => return err_404_prescription(&rx_id),
            Err(e) => return err_500(e),
        };
        match store::pills::list_by_prescription(&conn, &rx.id, store::ListFilter::All, &pagination)
        {
            Ok(page) => ok(page),
            Err(e) => err_500(e),
        }
    })
    .await
}
