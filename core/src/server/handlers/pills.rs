//! Handlers HTTP para la entidad Pill.

use axum::{
    extract::{Path, Query, State},
    Json,
};
use pillbox::{
    db::{self, store, store::registered_bottles},
    domain::{
        pill::{NewPill, PillPatch},
        search::SearchParams,
    },
    error::PillboxError,
};
use serde::Deserialize;
use serde_json::json;
use validator::Validate;

use super::{
    conn_for_bottle, err_400_invalid_id, err_404_pill, err_404_prescription, err_409,
    err_409_ambiguous_id, err_422, err_500, ok, ok_created, open_global_conn, ApiResponse,
    AppState,
};

/// Cuerpo JSON para crear una pill nueva via REST API.
#[derive(Deserialize, Validate)]
pub struct NewPillBody {
    #[validate(length(min = 1, max = 255))]
    pub title: String,
    #[validate(length(min = 1, max = 5000))]
    pub content: String,
    #[validate(length(min = 1, max = 64))]
    pub compound: String,
    pub author_name: Option<String>,
    pub author_email: Option<String>,
}

/// Handler `POST /api/bottles/:id/prescriptions/:rx_id/pills` — crea una pill nueva.
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
    let full_rx_id = match store::prescriptions::read_any(&conn, &rx_id) {
        Ok(Some(rx)) => rx.id,
        Ok(None) => return err_404_prescription(&rx_id),
        Err(e) => return err_500(e),
    };
    let new_pill = NewPill {
        title: input.title,
        content: input.content,
        compound: input.compound,
        prescription_id: full_rx_id,
        author_name: input.author_name,
        author_email: input.author_email,
    };
    match store::pills::take(&mut conn, &new_pill) {
        Ok(r) => ok_created(r),
        Err(e) => match e.downcast::<PillboxError>() {
            Ok(PillboxError::PrescriptionClosed { ref prescription_id }) => err_409(
                "prescription_closed",
                "prescription_closed",
                json!({ "prescription_id": prescription_id }),
            ),
            Ok(other) => err_500(other.into()),
            Err(e) => err_500(e),
        },
    }
}

/// Handler `GET /api/.../pills/:id` — lee una pill (incluyendo archivadas) por UUID.
pub async fn pill_get(
    State(s): State<AppState>,
    Path((bottle_id, _rx_id, pill_id)): Path<(String, String, String)>,
) -> ApiResponse {
    let conn = match conn_for_bottle(&s, &bottle_id) {
        Ok(c) => c,
        Err(r) => return r,
    };
    match store::pills::read_any(&conn, &pill_id) {
        Ok(Some(p)) => ok(p),
        Ok(None) => err_404_pill(pill_id),
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

/// Handler `PATCH /api/.../pills/:id` — aplica un patch parcial sobre una pill.
pub async fn pill_patch(
    State(s): State<AppState>,
    Path((bottle_id, _rx_id, pill_id)): Path<(String, String, String)>,
    Json(patch): Json<PillPatch>,
) -> ApiResponse {
    if let Err(e) = patch.validate() {
        return err_422(e);
    }
    let mut conn = match conn_for_bottle(&s, &bottle_id) {
        Ok(c) => c,
        Err(r) => return r,
    };
    match store::pills::revise(&mut conn, &pill_id, &patch) {
        Ok(Some(p)) => ok(p),
        Ok(None) => err_404_pill(pill_id),
        Err(e) => match e.downcast::<PillboxError>() {
            Ok(PillboxError::PrescriptionClosed { ref prescription_id }) => err_409(
                "prescription_closed",
                "prescription_closed",
                json!({ "prescription_id": prescription_id }),
            ),
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

/// Handler `DELETE /api/.../pills/:id` — realiza un soft delete de una pill.
pub async fn pill_delete(
    State(s): State<AppState>,
    Path((bottle_id, _rx_id, pill_id)): Path<(String, String, String)>,
) -> ApiResponse {
    let mut conn = match conn_for_bottle(&s, &bottle_id) {
        Ok(c) => c,
        Err(r) => return r,
    };
    match store::pills::discard(&mut conn, &pill_id) {
        Ok(Some(r)) => ok(r),
        Ok(None) => err_404_pill(pill_id),
        Err(e) => match e.downcast::<PillboxError>() {
            Ok(PillboxError::PrescriptionClosed { ref prescription_id }) => err_409(
                "prescription_closed",
                "prescription_closed",
                json!({ "prescription_id": prescription_id }),
            ),
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

/// Handler `DELETE /api/.../pills/:id/purge` — elimina permanentemente una pill.
pub async fn pill_purge(
    State(s): State<AppState>,
    Path((bottle_id, _rx_id, pill_id)): Path<(String, String, String)>,
) -> ApiResponse {
    let mut conn = match conn_for_bottle(&s, &bottle_id) {
        Ok(c) => c,
        Err(r) => return r,
    };
    match store::pills::hard_delete(&mut conn, &pill_id) {
        Ok(Some(_)) => ok(json!({ "purged": true })),
        Ok(None) => err_404_pill(pill_id),
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

/// Handler `GET /api/pills/search` — busca pills con FTS5 en una o todas las DBs registradas.
///
/// Si `bottle_id` está presente busca solo en esa DB; si no, agrega resultados de todas
/// las DBs registradas y los ordena por relevancia.
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
