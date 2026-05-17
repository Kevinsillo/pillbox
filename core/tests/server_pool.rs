//! Integration test: el servidor HTTP de `pillbox serve` sobrevive a una
//! ráfaga de requests concurrentes sin "database is locked" ni 500s.
//!
//! Levanta el server in-process bindeando `127.0.0.1:0` (puerto efímero) y
//! pasando el `TcpListener` ya bindeado a `pillbox::server::run_with_listener`.
//! Esto evita el coste de spawnear un subproceso por test y ejercita la misma
//! API pública que el binario (`cmd_serve_run` también pasa por aquí).

use std::time::Duration;

use serde_json::Value;

use pillbox::server::{run_with_listener, AppState};

/// Construye un server in-process aislado: tempdir como HOME, DB global y local
/// creadas a mano, listener en puerto efímero. Devuelve la URL base y guarda el
/// tempdir vivo hasta el final del test.
struct ServerHandle {
    base_url: String,
    _tmp: tempfile::TempDir,
}

async fn spawn_server() -> ServerHandle {
    let tmp = tempfile::tempdir().unwrap();

    // DB global y local en disco — los handlers asumen rutas reales para
    // resolver bottle paths desde `registered_bottles`. Ambas migran en open().
    let global_db = tmp.path().join("global.db");
    let local_db = tmp.path().join("local.db");
    {
        let _ = pillbox::db::connection::open(&global_db, pillbox::db::DbScope::Global)
            .expect("seed global DB");
        let _ = pillbox::db::connection::open(&local_db, pillbox::db::DbScope::Local)
            .expect("seed local DB");
    }

    let state = AppState::build(local_db, global_db).expect("build AppState");

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind ephemeral port");
    let port = listener.local_addr().unwrap().port();

    tokio::spawn(async move {
        let _ = run_with_listener(listener, state).await;
    });

    // El listener ya está bindeado antes del spawn → axum::serve acepta
    // conexiones desde la primera iteración del loop. Un pequeño yield basta
    // para dejar al spawn empezar a aceptar.
    tokio::task::yield_now().await;

    ServerHandle {
        base_url: format!("http://127.0.0.1:{}", port),
        _tmp: tmp,
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn ten_concurrent_info_requests_succeed() {
    let server = spawn_server().await;
    let client = reqwest::Client::new();

    let mut handles = Vec::with_capacity(10);
    for _ in 0..10 {
        let client = client.clone();
        let url = format!("{}/api/info", server.base_url);
        handles.push(tokio::spawn(async move {
            let resp = client.get(&url).send().await.expect("send");
            let status = resp.status();
            let body: Value = resp.json().await.expect("json");
            (status, body)
        }));
    }

    for h in handles {
        let (status, body) = h.await.expect("join");
        assert_eq!(status, reqwest::StatusCode::OK, "body={body}");
        assert_eq!(body["ok"], true, "body={body}");
        assert!(body["data"]["version"].is_string());
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn hundred_concurrent_info_requests_succeed() {
    // Cobertura de Tarea 3.18: bajo 100 requests concurrentes el pool global
    // debe reutilizar conexiones (POOL_MAX_SIZE = 8) sin 500s ni timeouts.
    // Si el pool no reusara, con 8 conexiones máximo y `busy_timeout=5s`
    // veríamos 500s o latencia patológica.
    let server = spawn_server().await;
    let client = reqwest::Client::new();

    let mut handles = Vec::with_capacity(100);
    let start = std::time::Instant::now();
    for _ in 0..100 {
        let client = client.clone();
        let url = format!("{}/api/info", server.base_url);
        handles.push(tokio::spawn(async move {
            let resp = client.get(&url).send().await.expect("send");
            let status = resp.status();
            let body: Value = resp.json().await.expect("json");
            (status, body)
        }));
    }

    for h in handles {
        let (status, body) = h.await.expect("join");
        assert_eq!(status, reqwest::StatusCode::OK, "body={body}");
        assert_eq!(body["ok"], true, "body={body}");
    }
    let elapsed = start.elapsed();
    assert!(
        elapsed < Duration::from_secs(10),
        "100 concurrent /info took {:?} (>10s suggests pool not reusing)",
        elapsed
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn ten_concurrent_bottles_list_requests_succeed() {
    // Endpoint que toca el global_pool (registered_bottles::list). Verifica
    // que el pool no bloquea bajo concurrencia. DB vacía → respuesta
    // legítima con `items=[]`, sin 500.
    let server = spawn_server().await;
    let client = reqwest::Client::new();

    let mut handles = Vec::with_capacity(10);
    for _ in 0..10 {
        let client = client.clone();
        let url = format!("{}/api/bottles", server.base_url);
        handles.push(tokio::spawn(async move {
            let resp = client.get(&url).send().await.expect("send");
            let status = resp.status();
            let body: Value = resp.json().await.expect("json");
            (status, body)
        }));
    }

    for h in handles {
        let (status, body) = h.await.expect("join");
        assert_eq!(status, reqwest::StatusCode::OK, "body={body}");
        assert_eq!(body["ok"], true, "body={body}");
    }
}
