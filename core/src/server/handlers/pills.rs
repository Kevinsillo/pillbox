//! Handlers HTTP para la entidad Pill.
//!
//! Las queries rusqlite van envueltas en `spawn_blocking` para no bloquear el
//! runtime axum.

use crate::{
    db::{store, store::registered_bottles},
    domain::{
        pill::{NewPill, PillPatch},
        search::SearchParams,
        Paginated, PaginationParams,
    },
    error::PillboxError,
};
use axum::{
    extract::{Path, Query, State},
    Json,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use validator::Validate;

use super::{
    blocking, bottle_pool_for, conn_for_bottle, err_400_invalid_id, err_400_pagination,
    err_404_pill, err_404_prescription, err_409, err_409_ambiguous_id, err_422, err_500, ok,
    ok_created, open_global_conn, ApiResponse, AppState,
};

/// Parámetros de query para `GET /api/pills/search` — combina `SearchParams` con paginación.
#[derive(Deserialize)]
pub struct PillSearchQuery {
    #[serde(flatten)]
    pub search: SearchParams,
    #[serde(flatten)]
    pub pagination: PaginationParams,
}

/// Parámetros de query para `GET /api/pills/compounds`.
#[derive(Deserialize)]
pub struct CompoundsQuery {
    pub bottle_id: Option<String>,
    pub limit: Option<u32>,
}

/// Entrada de la respuesta de `compounds`: `{compound, count}`.
#[derive(Serialize)]
pub struct CompoundEntry {
    pub compound: String,
    pub count: i64,
}

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
    blocking(move || {
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
                Ok(PillboxError::PrescriptionClosed {
                    ref prescription_id,
                }) => err_409(
                    "prescription_closed",
                    "prescription_closed",
                    json!({ "prescription_id": prescription_id }),
                ),
                Ok(other) => err_500(other.into()),
                Err(e) => err_500(e),
            },
        }
    })
    .await
}

/// Handler `GET /api/.../pills/:id` — lee una pill (incluyendo archivadas) por UUID.
pub async fn pill_get(
    State(s): State<AppState>,
    Path((bottle_id, _rx_id, pill_id)): Path<(String, String, String)>,
) -> ApiResponse {
    blocking(move || {
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
    })
    .await
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
    blocking(move || {
        let mut conn = match conn_for_bottle(&s, &bottle_id) {
            Ok(c) => c,
            Err(r) => return r,
        };
        match store::pills::revise(&mut conn, &pill_id, &patch) {
            Ok(Some(p)) => ok(p),
            Ok(None) => err_404_pill(pill_id),
            Err(e) => match e.downcast::<PillboxError>() {
                Ok(PillboxError::PrescriptionClosed {
                    ref prescription_id,
                }) => err_409(
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
    })
    .await
}

/// Handler `DELETE /api/.../pills/:id` — realiza un soft delete de una pill.
pub async fn pill_delete(
    State(s): State<AppState>,
    Path((bottle_id, _rx_id, pill_id)): Path<(String, String, String)>,
) -> ApiResponse {
    blocking(move || {
        let mut conn = match conn_for_bottle(&s, &bottle_id) {
            Ok(c) => c,
            Err(r) => return r,
        };
        match store::pills::discard(&mut conn, &pill_id) {
            Ok(Some(r)) => ok(r),
            Ok(None) => err_404_pill(pill_id),
            Err(e) => match e.downcast::<PillboxError>() {
                Ok(PillboxError::PrescriptionClosed {
                    ref prescription_id,
                }) => err_409(
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
    })
    .await
}

/// Handler `DELETE /api/.../pills/:id/purge` — elimina permanentemente una pill.
pub async fn pill_purge(
    State(s): State<AppState>,
    Path((bottle_id, _rx_id, pill_id)): Path<(String, String, String)>,
) -> ApiResponse {
    blocking(move || {
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
    })
    .await
}

/// Handler `GET /api/pills/search` — busca pills con FTS5 en una o todas las DBs registradas.
///
/// Si `bottle_id` está presente busca solo en esa DB; si no, agrega resultados de todas
/// las DBs registradas y los ordena por relevancia.
pub async fn pill_search(
    State(s): State<AppState>,
    Query(q): Query<PillSearchQuery>,
) -> ApiResponse {
    if let Err(e) = q.pagination.validate() {
        return err_400_pagination(&e);
    }
    let PillSearchQuery {
        search: params,
        pagination,
    } = q;

    blocking(move || {
        if let Some(ref bottle_id) = params.bottle_id {
            let conn = match conn_for_bottle(&s, bottle_id) {
                Ok(c) => c,
                Err(r) => return r,
            };
            match store::search::pill_find(&conn, &params, &pagination) {
                Ok(page) => ok(page),
                Err(e) => err_500(e),
            }
        } else {
            // Sin bottle_id: agregamos resultados de todas las DBs registradas.
            //
            // Limitación conocida: como cada DB pagina de forma independiente, no
            // podemos obtener "la página N" globalmente sin sobre-fetchear. Por
            // simplicidad y para mantener consistencia con el comportamiento
            // anterior, pedimos a cada DB un window grande (page=1, page_size=100)
            // y aplicamos slicing en memoria sobre el resultado combinado.
            // `total` es la suma de los totales reportados por cada DB.
            let global_conn = match open_global_conn(&s) {
                Ok(c) => c,
                Err(r) => return r,
            };
            let registered = match registered_bottles::list(&global_conn).map_err(err_500) {
                Ok(r) => r,
                Err(r) => return r,
            };
            drop(global_conn);
            let wide = PaginationParams {
                page: 1,
                page_size: 100,
            };
            let mut all_results = vec![];
            let mut total: u64 = 0;
            let mut used_fuzzy = false;
            for reg in &registered {
                let path = std::path::PathBuf::from(&reg.db_path);
                if !path.exists() {
                    continue;
                }
                let pool = match bottle_pool_for(&s, &path) {
                    Ok(p) => p,
                    Err(_) => continue,
                };
                let conn = match pool.get() {
                    Ok(c) => c,
                    Err(_) => continue,
                };
                if let Ok(page) = store::search::pill_find(&conn, &params, &wide) {
                    total = total.saturating_add(page.total);
                    used_fuzzy |= page.used_fuzzy;
                    all_results.extend(page.items);
                }
            }
            all_results.sort_by(|a, b| {
                a.rank
                    .partial_cmp(&b.rank)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
            let offset = pagination.offset() as usize;
            let limit = pagination.limit() as usize;
            let items = all_results.into_iter().skip(offset).take(limit).collect();
            ok(Paginated {
                items,
                total,
                page: pagination.page,
                page_size: pagination.page_size,
                used_fuzzy,
            })
        }
    })
    .await
}

/// Handler `GET /api/pills/compounds` — devuelve los compounds distintos y su conteo.
///
/// Si `bottle_id` está presente filtra por esa DB; si no, agrega todas las DBs
/// registradas. `limit` por defecto 50, capeado a 200.
pub async fn compounds(
    State(s): State<AppState>,
    Query(params): Query<CompoundsQuery>,
) -> ApiResponse {
    let limit = params.limit.unwrap_or(50).min(200);

    blocking(move || {
        if let Some(ref bottle_id) = params.bottle_id {
            let conn = match conn_for_bottle(&s, bottle_id) {
                Ok(c) => c,
                Err(r) => return r,
            };
            match store::pills::distinct_compounds(&conn, Some(bottle_id.as_str()), limit) {
                Ok(rows) => {
                    let entries: Vec<CompoundEntry> = rows
                        .into_iter()
                        .map(|(compound, count)| CompoundEntry { compound, count })
                        .collect();
                    ok(entries)
                }
                Err(e) => err_500(e),
            }
        } else {
            // Sin bottle_id: agrega resultados de todas las DBs registradas.
            let global_conn = match open_global_conn(&s) {
                Ok(c) => c,
                Err(r) => return r,
            };
            let registered = match registered_bottles::list(&global_conn).map_err(err_500) {
                Ok(r) => r,
                Err(r) => return r,
            };
            drop(global_conn);
            let mut agg: std::collections::HashMap<String, i64> = std::collections::HashMap::new();
            for reg in &registered {
                let path = std::path::PathBuf::from(&reg.db_path);
                if !path.exists() {
                    continue;
                }
                let pool = match bottle_pool_for(&s, &path) {
                    Ok(p) => p,
                    Err(_) => continue,
                };
                let conn = match pool.get() {
                    Ok(c) => c,
                    Err(_) => continue,
                };
                if let Ok(rows) = store::pills::distinct_compounds(&conn, None, limit) {
                    for (compound, count) in rows {
                        *agg.entry(compound).or_insert(0) += count;
                    }
                }
            }
            let mut entries: Vec<CompoundEntry> = agg
                .into_iter()
                .map(|(compound, count)| CompoundEntry { compound, count })
                .collect();
            entries.sort_by(|a, b| {
                b.count
                    .cmp(&a.count)
                    .then_with(|| a.compound.cmp(&b.compound))
            });
            entries.truncate(limit as usize);
            ok(entries)
        }
    })
    .await
}
