//! Handlers HTTP para la entidad Bottle y registered_bottles.

use axum::{
    extract::{Path, Query, State},
    Json,
};
use pillbox::{
    db::{self, store, store::registered_bottles},
    domain::{
        bottle::{Bottle, NewBottle},
        PaginationParams,
    },
    error::PillboxError,
};
use serde::{Deserialize, Serialize};
use validator::Validate;

use super::{
    conn_for_bottle, conn_for_bottle_with_id, default_30, err_400_invalid_id,
    err_400_pagination, err_404_bottle, err_404_registered_bottle, err_409_ambiguous_id, err_422,
    err_500, ok, ok_created, open_global_conn, ApiResponse, AppState,
};

/// Handler `GET /api/bottles/:id` — devuelve un bottle por su UUID.
pub async fn bottle_get(State(s): State<AppState>, Path(id): Path<String>) -> ApiResponse {
    let conn = match conn_for_bottle(&s, &id) {
        Ok(c) => c,
        Err(r) => return r,
    };
    match store::bottles::find_by_id(&conn, &id) {
        Ok(Some(b)) => ok(b),
        Ok(None) => err_404_bottle(&id),
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

/// Handler `GET /api/bottles` — lista todos los bottles registrados en la DB global.
///
/// Acepta `?page=&page_size=`. Como agregamos bottles de múltiples DBs locales,
/// recolectamos todo, ordenamos por display_name y aplicamos slicing en memoria
/// para devolver la página solicitada con el `total` real.
pub async fn bottle_list(
    State(s): State<AppState>,
    Query(pagination): Query<PaginationParams>,
) -> ApiResponse {
    if let Err(e) = pagination.validate() {
        return err_400_pagination(&e);
    }
    let global_conn = match open_global_conn(&s) {
        Ok(c) => c,
        Err(r) => return r,
    };

    // registered_bottles es la única fuente de verdad (cubre locales y globales).
    let registered = match registered_bottles::list(&global_conn) {
        Ok(r) => r,
        Err(e) => return err_500(e),
    };

    let mut all_bottles: Vec<Bottle> = Vec::new();

    for reg in registered {
        let reg_db_path = std::path::Path::new(&reg.db_path);
        if !reg_db_path.exists() {
            let directory = reg_db_path
                .parent()
                .and_then(|p| p.parent())
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_else(|| reg.db_path.clone());
            all_bottles.push(Bottle {
                id: String::new(),
                name: reg.name,
                display_name: reg.display_name,
                directory,
                scope: "local".to_string(),
                created_at: reg.registered_at,
                last_seen_at: reg.last_seen_at,
                linked: false,
                reg_id: Some(reg.id),
            });
            continue;
        }
        let db_conn = match db::connection::open_existing(reg_db_path) {
            Ok(c) => c,
            Err(_) => continue,
        };
        let bottles = match store::bottles::list(&db_conn, &PaginationParams { page: 1, page_size: 100 }) {
            Ok(b) => b.items,
            Err(_) => continue,
        };
        for mut bottle in bottles {
            bottle.reg_id = Some(reg.id);
            all_bottles.push(bottle);
        }
    }

    all_bottles.sort_by(|a, b| a.display_name.cmp(&b.display_name));
    let total = all_bottles.len() as u64;
    let offset = pagination.offset() as usize;
    let limit = pagination.limit() as usize;
    let items: Vec<Bottle> = all_bottles.into_iter().skip(offset).take(limit).collect();
    ok(pillbox::domain::Paginated {
        items,
        total,
        page: pagination.page,
        page_size: pagination.page_size,
    })
}

/// Handler `POST /api/bottles` — crea un bottle nuevo y lo registra en la DB global.
pub async fn bottle_create(State(s): State<AppState>, Json(input): Json<NewBottle>) -> ApiResponse {
    if let Err(e) = input.validate() {
        return err_422(e);
    }
    let mut conn = match db::connection::open(&s.db_path).map_err(err_500) {
        Ok(c) => c,
        Err(r) => return r,
    };
    let bottle = match store::bottles::create(&mut conn, &input) {
        Ok(b) => b,
        Err(e) => return err_500(e),
    };
    // Registrar en la DB global para que conn_for_bottle pueda resolverlo.
    if let Ok(global_conn) = open_global_conn(&s) {
        let _ = registered_bottles::register(
            &global_conn,
            &bottle.id,
            &bottle.name,
            &bottle.display_name,
            &s.db_path.to_string_lossy(),
        );
    }
    ok_created(bottle)
}

/// Cuerpo JSON para actualizar la ruta de un registered_bottle.
#[derive(Deserialize, Serialize)]
pub struct UpdateRegisteredBottleBody {
    pub db_path: String,
}

/// Handler `PATCH /api/registered_bottles/:id` — actualiza la ruta de DB de un registro.
pub async fn registered_bottle_patch(
    State(s): State<AppState>,
    Path(id): Path<String>,
    Json(body): Json<UpdateRegisteredBottleBody>,
) -> ApiResponse {
    let db_file = std::path::Path::new(&body.db_path);
    if !db_file.is_file() {
        return super::err(
            axum::http::StatusCode::UNPROCESSABLE_ENTITY,
            "db_path_not_found",
            "The specified path does not exist or is not a file",
        );
    }
    let reg_id: i64 = match id.parse() {
        Ok(n) => n,
        Err(_) => return err_404_registered_bottle(&id),
    };
    let global_conn = match open_global_conn(&s) {
        Ok(c) => c,
        Err(r) => return r,
    };
    match registered_bottles::update_db_path(&global_conn, reg_id, &body.db_path) {
        Ok(true) => ok(serde_json::Value::Null),
        Ok(false) => err_404_registered_bottle(&id),
        Err(e) => err_500(e),
    }
}

/// Handler `DELETE /api/registered_bottles/:id` — elimina un registro de la DB global.
pub async fn registered_bottle_delete(
    State(s): State<AppState>,
    Path(id): Path<String>,
) -> ApiResponse {
    let reg_id: i64 = match id.parse() {
        Ok(n) => n,
        Err(_) => return err_404_registered_bottle(&id),
    };
    let global_conn = match open_global_conn(&s) {
        Ok(c) => c,
        Err(r) => return r,
    };
    match registered_bottles::unregister(&global_conn, reg_id) {
        Ok(true) => ok(serde_json::Value::Null),
        Ok(false) => err_404_registered_bottle(&id),
        Err(e) => err_500(e),
    }
}

/// Handler `DELETE /api/bottles/:id` — elimina un bottle y lo desregistra de la DB global.
pub async fn bottle_delete(State(s): State<AppState>, Path(id): Path<String>) -> ApiResponse {
    let mut conn = match conn_for_bottle(&s, &id) {
        Ok(c) => c,
        Err(r) => return r,
    };
    match store::bottles::delete(&mut conn, &id) {
        Ok(true) => {
            if let Ok(global_conn) = open_global_conn(&s) {
                if let Ok(Some(reg)) = registered_bottles::find_by_bottle_id(&global_conn, &id) {
                    let _ = registered_bottles::unregister(&global_conn, reg.id);
                }
            }
            ok(serde_json::Value::Null)
        }
        Ok(false) => err_404_bottle(&id),
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

/// Parámetros de query para las estadísticas de un bottle.
#[derive(Deserialize)]
pub struct BottleStatsParams {
    #[serde(default = "default_30")]
    pub days: u32,
}

/// Handler `GET /api/bottles/:id/stats` — devuelve estadísticas de actividad del bottle.
pub async fn bottle_stats(
    State(s): State<AppState>,
    Path(id): Path<String>,
    Query(params): Query<BottleStatsParams>,
) -> ApiResponse {
    let conn = match conn_for_bottle(&s, &id) {
        Ok(c) => c,
        Err(r) => return r,
    };
    let open_rx_pill_count = match store::pills::open_rx_pill_count(&conn) {
        Ok(n) => n,
        Err(e) => return err_500(e),
    };
    let closed_rx_count: i64 = match conn.query_row(
        "SELECT COUNT(*) FROM prescriptions WHERE ended_at IS NOT NULL AND deleted_at IS NULL",
        [],
        |row| row.get(0),
    ) {
        Ok(n) => n,
        Err(e) => return err_500(anyhow::anyhow!(e)),
    };
    let pills_per_day = match store::pills::activity_by_day(&conn, params.days) {
        Ok(v) => v,
        Err(e) => return err_500(e),
    };
    ok(serde_json::json!({
        "open_rx_pill_count": open_rx_pill_count,
        "closed_rx_count": closed_rx_count,
        "pills_per_day": pills_per_day,
    }))
}

/// Handler `GET /api/bottles/:id/prescriptions` — lista las prescripciones del bottle.
///
/// Acepta `?page=&page_size=`. Sólo devuelve prescripciones activas.
pub async fn bottle_prescriptions(
    State(s): State<AppState>,
    Path(id): Path<String>,
    Query(pagination): Query<PaginationParams>,
) -> ApiResponse {
    if let Err(e) = pagination.validate() {
        return err_400_pagination(&e);
    }
    let (conn, full_id) = match conn_for_bottle_with_id(&s, &id) {
        Ok(v) => v,
        Err(r) => return r,
    };
    match store::prescriptions::list_by_bottle(&conn, &full_id, store::ListFilter::Active, &pagination) {
        Ok(page) => ok(page),
        Err(e) => err_500(e),
    }
}
