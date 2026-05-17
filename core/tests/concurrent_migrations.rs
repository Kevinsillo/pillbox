//! Integration test: varios threads inicializan la misma DB en paralelo.
//!
//! Reproduce el escenario que el binario `pillbox` ya sufre en producción
//! cuando varios subcomandos (o el MCP one-shot + un comando del CLI) abren
//! la DB al mismo tiempo: cada uno aplica `migrations::run` sobre su propia
//! `Connection` apuntando al mismo fichero.
//!
//! El test cubre el cambio aplicado en Tarea 1.1 (BEGIN IMMEDIATE + re-check
//! bajo lock). Con `BEGIN DEFERRED` y sin re-check fallaba bajo carga
//! concurrente con `SQLITE_BUSY` o `UNIQUE constraint failed` sobre
//! `schema_migrations`.
//!
//! Threads en vez de procesos: el lock de SQLite (RESERVED) se adquiere a
//! nivel de fichero, así que se comporta igual entre threads que entre
//! procesos siempre que cada uno tenga su propia `Connection` (que es lo
//! que hacemos). Esto evita la fragilidad de spawnear binarios.

use std::sync::Arc;
use std::thread;

use rusqlite::Connection;

use pillbox::db::{migrations, DbScope};

/// Aplica los PRAGMA por conexión. WAL se asume ya activado por el caller
/// del test (ver `run_concurrent`): cambiar `journal_mode` requiere un
/// lock exclusivo que se serializa fuera de la ventana de carrera que
/// queremos ejercitar (la del runner de migraciones).
fn configure_per_conn(conn: &Connection) {
    conn.execute_batch(
        "PRAGMA busy_timeout  = 5000;
         PRAGMA synchronous   = NORMAL;
         PRAGMA foreign_keys  = ON;",
    )
    .unwrap();
}

fn run_concurrent(scope: DbScope) {
    let dir = tempfile::tempdir().unwrap();
    let path = Arc::new(dir.path().join("pillbox.db"));

    // Pre-activar WAL una sola vez. En producción esto lo hace
    // `connection::configure` en cada apertura, pero el cambio de
    // `journal_mode` solo se aplica de verdad la primera vez (idempotente
    // a partir de ahí). El binario funciona porque los procesos
    // concurrentes acaban viendo la DB ya en WAL; en el test forzamos
    // ese estado antes de la carrera para que la ventana bajo prueba sea
    // SOLO la del runner de migraciones.
    {
        let conn = Connection::open(&*path).unwrap();
        conn.execute_batch("PRAGMA journal_mode = WAL;").unwrap();
    }

    const N: usize = 4;
    let mut handles = Vec::with_capacity(N);

    for _ in 0..N {
        let path = Arc::clone(&path);
        let s = scope;
        handles.push(thread::spawn(move || -> anyhow::Result<()> {
            let conn = Connection::open(&*path)?;
            configure_per_conn(&conn);
            migrations::run(&conn, s)?;
            Ok(())
        }));
    }

    for h in handles {
        h.join()
            .expect("thread panicked")
            .expect("migration failed");
    }

    // Verificar estado final: una única fila en schema_migrations con
    // version == CURRENT_SCHEMA_VERSION.
    let conn = Connection::open(&*path).unwrap();
    let version: i64 = conn
        .query_row("SELECT MAX(version) FROM schema_migrations", [], |r| {
            r.get(0)
        })
        .unwrap();
    assert_eq!(version, migrations::CURRENT_SCHEMA_VERSION);

    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM schema_migrations", [], |r| r.get(0))
        .unwrap();
    assert_eq!(
        count, 1,
        "schema_migrations debe tener una sola fila (idempotencia bajo concurrencia)"
    );
}

#[test]
fn concurrent_migrations_local() {
    run_concurrent(DbScope::Local);
}

#[test]
fn concurrent_migrations_global() {
    run_concurrent(DbScope::Global);
}
