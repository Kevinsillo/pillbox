//! Servidor HTTP para `pillbox serve`.
//!
//! Expone la misma lógica que `pillbox exec` como una REST API en localhost:4242.
//! Cada request abre una conexión SQLite nueva — WAL mode lo soporta sin pool.
//! Al arrancar publica un servicio mDNS `pillbox._http._tcp.local.` para
//! descubrimiento en la red local.

mod assets;
mod handlers;

use std::{net::SocketAddr, path::PathBuf, sync::Arc};

use anyhow::Result;
use axum::{
    routing::{delete, get, patch, post},
    Router,
};
use mdns_sd::{ServiceDaemon, ServiceInfo};
use tower_http::cors::{Any, CorsLayer};

/// Estado compartido entre todos los handlers.
#[derive(Clone)]
pub struct AppState {
    pub db_path: Arc<PathBuf>,
    pub global_db_path: Arc<PathBuf>,
}

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

    // Publicar servicio mDNS (no fatal si falla)
    let _mdns = register_mdns(port);

    println!("pillbox serve en http://localhost:{}", port);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    // _mdns se dropea aquí → unregister automático del servicio mDNS
    Ok(())
}

/// Señal de apagado graceful: espera CTRL+C.
async fn shutdown_signal() {
    tokio::signal::ctrl_c()
        .await
        .expect("failed to install CTRL+C handler");
    println!("\npillbox serve detenido.");
}

/// Registra el servicio mDNS `pillbox._http._tcp.local.` en el puerto dado.
///
/// Devuelve el `ServiceDaemon` para mantenerlo vivo mientras dure el servidor.
/// Si mDNS no está disponible (sin soporte de red, permisos, etc.) registra un
/// warning y devuelve `None` — el servidor arranca igualmente.
fn register_mdns(port: u16) -> Option<ServiceDaemon> {
    let mdns = match ServiceDaemon::new() {
        Ok(d) => d,
        Err(e) => {
            tracing::warn!("mDNS: failed to create daemon: {}", e);
            return None;
        }
    };

    let hostname = system_hostname();
    let host_fqdn = format!("{}.local.", hostname);
    let local_ip = local_ipv4().unwrap_or_else(|| "127.0.0.1".to_string());

    let info = match ServiceInfo::new(
        "_http._tcp.local.",
        "pillbox",
        &host_fqdn,
        local_ip.as_str(),
        port,
        None,
    ) {
        Ok(i) => i,
        Err(e) => {
            tracing::warn!("mDNS: failed to create ServiceInfo: {}", e);
            return None;
        }
    };

    match mdns.register(info) {
        Ok(_) => {
            println!("mDNS:  pillbox._http._tcp.local. → {}:{}", local_ip, port);
            Some(mdns)
        }
        Err(e) => {
            tracing::warn!("mDNS: failed to register service: {}", e);
            None
        }
    }
}

/// Hostname del sistema sin dominio.
fn system_hostname() -> String {
    std::fs::read_to_string("/etc/hostname")
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|_| "pillbox".to_string())
}

/// IP local primaria del sistema (sin enviar tráfico real).
///
/// Abre un socket UDP hacia una IP pública y lee la dirección local que
/// el SO eligió — truco estándar para obtener la IP de la interfaz activa.
fn local_ipv4() -> Option<String> {
    use std::net::UdpSocket;
    let socket = UdpSocket::bind("0.0.0.0:0").ok()?;
    socket.connect("8.8.8.8:80").ok()?;
    let addr = socket.local_addr().ok()?;
    Some(addr.ip().to_string())
}
