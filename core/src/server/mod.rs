//! Servidor HTTP para `pillbox serve`.
//!
//! Expone la misma lógica que `pillbox exec` como una REST API en localhost:4242.
//! Mantiene un pool r2d2 por DB para reutilizar conexiones entre requests y
//! evitar bloqueos por exclusión de WAL bajo concurrencia.

mod assets;
mod handlers;

use std::{
    net::SocketAddr,
    num::NonZeroUsize,
    path::PathBuf,
    sync::{Arc, Mutex},
};

use anyhow::Result;
use axum::{
    routing::{delete, get, patch, post},
    Router,
};
use lru::LruCache;
use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;
use rust_i18n::t;
use tower_http::cors::{Any, CorsLayer};

use crate::db::{
    connection::{build_pool, MAX_BOTTLE_POOLS},
    DbScope,
};

/// Estado compartido entre todos los handlers.
///
/// El pool global se construye eagerly en startup (`serve` siempre necesita
/// hablar con la DB global para resolver `registered_bottles`). Los pools
/// por-bottle se construyen lazy en el primer acceso (`lookup-or-create`)
/// y se mantienen en un LRU con cap `MAX_BOTTLE_POOLS`. Cuando un pool es
/// desalojado por uno nuevo, r2d2 lo Dropea y cierra sus conexiones; las
/// queries en vuelo siguen vivas porque sus `PooledConnection` mantienen
/// el Arc interno del pool.
#[derive(Clone)]
pub struct AppState {
    pub db_path: Arc<PathBuf>,
    pub global_db_path: Arc<PathBuf>,
    pub global_pool: Pool<SqliteConnectionManager>,
    pub bottle_pools: Arc<Mutex<LruCache<PathBuf, Pool<SqliteConnectionManager>>>>,
}

impl AppState {
    /// Construye un `AppState` listo para servir: pool global eager + LRU vacío
    /// para los pools por-bottle.
    ///
    /// # Errors
    ///
    /// Retorna error si la DB global no puede abrirse/migrarse.
    pub fn build(db_path: PathBuf, global_db_path: PathBuf) -> Result<Self> {
        let global_pool = build_pool(&global_db_path, DbScope::Global)?;
        Ok(Self {
            db_path: Arc::new(db_path),
            global_db_path: Arc::new(global_db_path),
            global_pool,
            bottle_pools: Arc::new(Mutex::new(LruCache::new(
                NonZeroUsize::new(MAX_BOTTLE_POOLS).expect("MAX_BOTTLE_POOLS must be > 0"),
            ))),
        })
    }
}

/// Construye el `Router` axum completo (WebUI + API) sobre el `state` dado.
///
/// Lo expone como helper público para que `run`, `run_with_listener` y los
/// tests integration in-process compartan el mismo wiring sin duplicar rutas.
pub fn build_router(state: AppState) -> Router {
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let api = Router::new()
        // Pills — búsqueda cross-cutting (bottle_id opcional como filtro)
        .route("/pills/search", get(handlers::pill_search))
        .route("/pills/compounds", get(handlers::compounds))
        // Capsules (globales, sin bottle)
        .route("/capsules", get(handlers::capsule_list))
        .route("/capsules", post(handlers::capsule_create))
        .route("/capsules/search", get(handlers::capsule_search))
        .route("/capsules/compounds", get(handlers::capsule_compounds))
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
        .route(
            "/bottles/:bottle_id/prescriptions/:rx_id/reopen",
            post(handlers::prescription_reopen),
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
        .route("/info", get(handlers::info_get))
        .with_state(state.clone());

    Router::new()
        .route("/", get(assets::serve_root))
        .nest("/api", api)
        // WebUI — catch-all para SPA routing (debe ir al final)
        .route("/*path", get(assets::serve))
        .layer(cors)
        .with_state(state)
}

/// Sirve el router axum sobre un `TcpListener` ya bindeado, con shutdown
/// graceful por CTRL+C / SIGTERM.
///
/// Permite que los tests integration construyan el server in-process bindeando
/// `127.0.0.1:0` y leyendo el puerto efímero antes de spawnear el future, sin
/// necesidad de levantar un subproceso `pillbox serve run`.
///
/// # Errors
///
/// Retorna error si `axum::serve` falla.
pub async fn run_with_listener(listener: tokio::net::TcpListener, state: AppState) -> Result<()> {
    let app = build_router(state);
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;
    Ok(())
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
    let state = AppState::build(db_path, global_db_path)?;

    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    let url = format!("http://localhost:{}", port);
    println!("\n{}", t!("serve.run.listening", url = url));

    let listener = tokio::net::TcpListener::bind(addr).await?;
    run_with_listener(listener, state).await
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

    println!("\n{}", t!("serve.run.stopped"));
}
