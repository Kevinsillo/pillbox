//! Servidor HTTP para `pillbox serve`.
//!
//! Expone la misma lógica que `pillbox exec` como una REST API en localhost:4242.
//! Cada request abre una conexión SQLite nueva — WAL mode lo soporta sin pool.

mod handlers;

use std::{net::SocketAddr, path::PathBuf, sync::Arc};

use anyhow::Result;
use axum::{
    routing::{delete, get, patch, post},
    Router,
};
use tower_http::cors::{Any, CorsLayer};

/// Estado compartido entre todos los handlers.
#[derive(Clone)]
pub struct AppState {
    pub db_path: Arc<PathBuf>,
}

pub async fn run(port: u16, db_path: PathBuf) -> Result<()> {
    let state = AppState { db_path: Arc::new(db_path) };

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let app = Router::new()
        // Pills
        .route("/pills",          post(handlers::pill_create))
        .route("/pills/search",   get(handlers::pill_search))
        .route("/pills/:id",      get(handlers::pill_get))
        .route("/pills/:id",      patch(handlers::pill_patch))
        .route("/pills/:id",      delete(handlers::pill_delete))
        // Capsules
        .route("/capsules",       post(handlers::capsule_create))
        .route("/capsules/search",get(handlers::capsule_search))
        .route("/capsules/:id",   get(handlers::capsule_get))
        .route("/capsules/:id",   patch(handlers::capsule_patch))
        .route("/capsules/:id",   delete(handlers::capsule_delete))
        // Prescriptions
        .route("/prescriptions",      post(handlers::prescription_open))
        .route("/prescriptions/:id",  get(handlers::prescription_get))
        .route("/prescriptions/:id",  patch(handlers::prescription_close))
        .route("/prescriptions/:id",  delete(handlers::prescription_delete))
        // Bottles
        .route("/bottles",        get(handlers::bottle_list))
        .route("/bottles",        post(handlers::bottle_create))
        // Context
        .route("/context",        get(handlers::context_get))
        .layer(cors)
        .with_state(state);

    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    tracing::info!("pillbox serve escuchando en http://{}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}
