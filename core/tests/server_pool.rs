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

// ─── Helpers para tests de cleanup de DB local al borrar el último bottle ────

/// Variante de `spawn_server` que además devuelve los paths de las DBs creadas
/// y el `AppState` para poder inspeccionar el LRU desde el test. El tempdir se
/// devuelve fuera porque su Drop borraría las DBs si saliera de scope.
struct CleanupHandle {
    base_url: String,
    local_db: std::path::PathBuf,
    global_db: std::path::PathBuf,
    state: pillbox::server::AppState,
    _tmp: tempfile::TempDir,
}

async fn spawn_server_for_cleanup() -> CleanupHandle {
    let tmp = tempfile::tempdir().unwrap();
    let global_db = tmp.path().join("global.db");
    let local_db = tmp.path().join("local.db");
    {
        let _ = pillbox::db::connection::open(&global_db, pillbox::db::DbScope::Global)
            .expect("seed global DB");
        let _ = pillbox::db::connection::open(&local_db, pillbox::db::DbScope::Local)
            .expect("seed local DB");
    }

    let state =
        pillbox::server::AppState::build(local_db.clone(), global_db.clone()).expect("build state");
    let state_for_server = state.clone();

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    tokio::spawn(async move {
        let _ = run_with_listener(listener, state_for_server).await;
    });
    tokio::task::yield_now().await;

    CleanupHandle {
        base_url: format!("http://127.0.0.1:{}", port),
        local_db,
        global_db,
        state,
        _tmp: tmp,
    }
}

/// Crea un bottle en la DB local indicada y lo registra en la DB global.
/// Devuelve el UUID del bottle.
fn seed_bottle(local_db: &std::path::Path, global_db: &std::path::Path) -> String {
    use pillbox::db::{connection, store, DbScope};
    use pillbox::domain::bottle::{BottleScope, NewBottle};

    let mut local_conn = connection::open(local_db, DbScope::Local).unwrap();
    let bottle = store::bottles::create(
        &mut local_conn,
        &NewBottle {
            name: "test-bottle".into(),
            display_name: "Test Bottle".into(),
            directory: "/tmp/fake-test-dir".into(),
            scope: BottleScope::Local,
        },
    )
    .unwrap();

    let global_conn = connection::open(global_db, DbScope::Global).unwrap();
    store::registered_bottles::register(
        &global_conn,
        &bottle.id,
        &bottle.name,
        &bottle.display_name,
        &local_db.to_string_lossy(),
    )
    .unwrap();

    bottle.id
}

// ─── 4.2 — DELETE HTTP borra el .db local y purga el LRU ─────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn http_delete_last_bottle_removes_local_db_and_evicts_pool() {
    let server = spawn_server_for_cleanup().await;
    let bottle_id = seed_bottle(&server.local_db, &server.global_db);

    // Forzar la creación del pool del bottle vía un GET previo, para asegurar
    // que el LRU contiene la entrada antes del DELETE.
    let client = reqwest::Client::new();
    let _ = client
        .get(format!("{}/api/bottles/{}", server.base_url, bottle_id))
        .send()
        .await
        .expect("get bottle");
    {
        let cache = server.state.bottle_pools.lock().unwrap();
        assert!(
            cache.contains(&server.local_db),
            "pool LRU should contain local_db after first access"
        );
    }

    assert!(server.local_db.exists(), "precondition: local db exists");

    let resp = client
        .delete(format!("{}/api/bottles/{}", server.base_url, bottle_id))
        .send()
        .await
        .expect("send DELETE");
    assert_eq!(resp.status(), reqwest::StatusCode::OK);

    // Pequeño yield para que la tarea blocking termine de aplicar el cleanup.
    tokio::task::yield_now().await;

    assert!(
        !server.local_db.exists(),
        "local db file should be removed after deleting last bottle"
    );
    // Sidecars también deben estar ausentes.
    let wal = server.local_db.with_file_name("local.db-wal");
    let shm = server.local_db.with_file_name("local.db-shm");
    assert!(!wal.exists(), "wal sidecar should be removed");
    assert!(!shm.exists(), "shm sidecar should be removed");

    // El pool del bottle se evicta del LRU.
    {
        let cache = server.state.bottle_pools.lock().unwrap();
        assert!(
            !cache.contains(&server.local_db),
            "pool LRU should not contain local_db after cleanup"
        );
    }
}

// ─── 4.4 — Segundo DELETE al mismo id devuelve 404 limpio ─────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn http_delete_is_idempotent_returns_404_second_time() {
    let server = spawn_server_for_cleanup().await;
    let bottle_id = seed_bottle(&server.local_db, &server.global_db);

    let client = reqwest::Client::new();
    let r1 = client
        .delete(format!("{}/api/bottles/{}", server.base_url, bottle_id))
        .send()
        .await
        .unwrap();
    assert_eq!(r1.status(), reqwest::StatusCode::OK, "primer DELETE OK");

    let r2 = client
        .delete(format!("{}/api/bottles/{}", server.base_url, bottle_id))
        .send()
        .await
        .unwrap();
    assert_eq!(
        r2.status(),
        reqwest::StatusCode::NOT_FOUND,
        "segundo DELETE debe devolver 404 limpio"
    );
    let body: Value = r2.json().await.unwrap();
    assert_eq!(body["ok"], false);
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
