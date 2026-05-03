//! Servidor HTTP para `pillbox serve`.
//!
//! Expone la misma lógica que `pillbox exec` como una REST API en localhost:4242.
//! Cada request abre una conexión SQLite nueva — WAL mode lo soporta sin pool.

mod assets;
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
    pub global_db_path: Arc<PathBuf>,
}

/// Arranca el servidor HTTP en `127.0.0.1:<port>`.
///
/// Cada request abre su propia conexión SQLite al `db_path` indicado; WAL mode
/// permite concurrencia sin pool. Se detiene gracefully al recibir CTRL+C o SIGTERM.
///
/// # Errors
///
/// Retorna error si el bind del puerto falla o si axum no puede servir conexiones.
pub async fn run(port: u16, db_path: PathBuf, global_db_path: PathBuf) -> Result<()> {
    let state = AppState {
        db_path: Arc::new(db_path),
        global_db_path: Arc::new(global_db_path),
    };

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let api = Router::new()
        // Pills — búsqueda cross-cutting (bottle_id opcional como filtro)
        .route("/pills/search", get(handlers::pill_search))
        // Capsules (globales, sin bottle)
        .route("/capsules", get(handlers::capsule_list))
        .route("/capsules", post(handlers::capsule_create))
        .route("/capsules/search", get(handlers::capsule_search))
        .route("/capsules/:id", get(handlers::capsule_get))
        .route("/capsules/:id", patch(handlers::capsule_patch))
        .route("/capsules/:id", delete(handlers::capsule_delete))
        .route("/capsules/:id/purge", delete(handlers::capsule_purge))
        // Bottles
        .route("/bottles", get(handlers::bottle_list))
        .route("/bottles", post(handlers::bottle_create))
        .route("/bottles/:bottle_id", get(handlers::bottle_get))
        .route("/bottles/:bottle_id", delete(handlers::bottle_delete))
        .route("/bottles/:bottle_id/context", get(handlers::context_get))
        .route("/bottles/:bottle_id/stats", get(handlers::bottle_stats))
        // Prescriptions (anidadas bajo bottle)
        .route(
            "/bottles/:bottle_id/prescriptions",
            get(handlers::bottle_prescriptions),
        )
        .route(
            "/bottles/:bottle_id/prescriptions",
            post(handlers::prescription_open),
        )
        .route(
            "/bottles/:bottle_id/prescriptions/:rx_id",
            get(handlers::prescription_get),
        )
        .route(
            "/bottles/:bottle_id/prescriptions/:rx_id",
            patch(handlers::prescription_close),
        )
        .route(
            "/bottles/:bottle_id/prescriptions/:rx_id",
            delete(handlers::prescription_delete),
        )
        .route(
            "/bottles/:bottle_id/prescriptions/:rx_id/purge",
            delete(handlers::prescription_purge),
        )
        // Pills (anidadas bajo prescription)
        .route(
            "/bottles/:bottle_id/prescriptions/:rx_id/pills",
            get(handlers::prescription_pills),
        )
        .route(
            "/bottles/:bottle_id/prescriptions/:rx_id/pills",
            post(handlers::pill_create),
        )
        .route(
            "/bottles/:bottle_id/prescriptions/:rx_id/pills/:pill_id",
            get(handlers::pill_get),
        )
        .route(
            "/bottles/:bottle_id/prescriptions/:rx_id/pills/:pill_id",
            patch(handlers::pill_patch),
        )
        .route(
            "/bottles/:bottle_id/prescriptions/:rx_id/pills/:pill_id",
            delete(handlers::pill_delete),
        )
        .route(
            "/bottles/:bottle_id/prescriptions/:rx_id/pills/:pill_id/purge",
            delete(handlers::pill_purge),
        )
        // Registered bottles (gestión de registros rotos)
        .route(
            "/registered_bottles/:id",
            patch(handlers::registered_bottle_patch),
        )
        .route(
            "/registered_bottles/:id",
            delete(handlers::registered_bottle_delete),
        )
        // Meta
        .route("/version", get(handlers::version_get))
        .with_state(state.clone());

    let app = Router::new()
        .route("/", get(assets::serve_root))
        .nest("/api", api)
        // WebUI — catch-all para SPA routing (debe ir al final)
        .route("/*path", get(assets::serve))
        .layer(cors)
        .with_state(state);

    let addr = SocketAddr::from(([127, 0, 0, 1], port));

    println!("pillbox serve en http://localhost:{}", port);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    Ok(())
}

/// Señal de apagado graceful: espera CTRL+C o SIGTERM (en Unix).
///
/// En sistemas Unix se combinan ambas señales con `tokio::select!` para que
/// systemd/launchd puedan detener el servicio de forma ordenada.
async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install CTRL+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        use tokio::signal::unix::{signal, SignalKind};
        signal(SignalKind::terminate())
            .expect("failed to install SIGTERM handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }

    println!("\npillbox serve detenido.");
}
