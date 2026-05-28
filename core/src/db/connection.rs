use std::path::Path;

use anyhow::{Context, Result};
use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::Connection;

use crate::db::{migrations, DbScope};

/// Tamaño máximo del pool por DB (servir/HTTP/MCP). Suficiente para los
/// handlers axum corrientes — los hot-paths van por `spawn_blocking`, así
/// que el pool sirve picos de concurrencia, no carga sostenida.
pub const POOL_MAX_SIZE: u32 = 8;

/// Cap del LRU `AppState.bottle_pools` (en `pillbox serve`).
///
/// Cada bottle local accedido por HTTP crea un pool r2d2 de hasta
/// `POOL_MAX_SIZE` conexiones SQLite. Sin cap, una sola sesión `serve` que
/// itere el registry (e.g. `bottle_list`) acumula pools de N bottles
/// indefinidamente.
///
/// 16 = 2× el flow típico (2-4 agentes MCP activos + WebUI sobre 1-2
/// bottles + slack). Cuando un pool se desaloja, r2d2 lo Dropea y cierra
/// sus conexiones; las queries en vuelo mantienen vivo el Arc interno a
/// través de su `PooledConnection`.
pub const MAX_BOTTLE_POOLS: usize = 16;

/// PRAGMA de configuración aplicados a cada conexión SQLite.
///
/// Fuente única de verdad compartida por `pragma_init` (pool, vía
/// `with_init`) y `configure` (conexiones directas). Los PRAGMA no
/// persisten entre conexiones en SQLite, así que se reaplican en cada
/// apertura.
const PRAGMA_SQL: &str = "PRAGMA journal_mode  = WAL;
         PRAGMA busy_timeout  = 5000;
         PRAGMA synchronous   = NORMAL;
         PRAGMA foreign_keys  = ON;
         PRAGMA auto_vacuum   = INCREMENTAL;
         PRAGMA cache_size    = -32768;";

/// Closure de inicialización aplicada a cada nueva conexión del pool.
///
/// Centraliza los PRAGMA en un sólo sitio (DRY frente a `configure`) y
/// permite que `SqliteConnectionManager::with_init` los aplique.
fn pragma_init(conn: &mut Connection) -> rusqlite::Result<()> {
    conn.execute_batch(PRAGMA_SQL)
}

/// Construye un pool r2d2 sobre la DB en `path` y ejecuta las migraciones
/// del `scope` indicado **una vez**.
///
/// Cada conexión del pool aplica los mismos PRAGMA que `configure`
/// (WAL, busy_timeout=5000, synchronous=NORMAL, foreign_keys=ON,
/// auto_vacuum=INCREMENTAL, cache_size=-32768). La migración corre sobre la primera
/// conexión adquirida; gracias a `migrations::run` con `BEGIN IMMEDIATE`
/// es segura ante concurrencia multi-proceso, así que aunque la abramos
/// también en otros procesos serializa correctamente.
///
/// # Errors
///
/// Retorna error si no puede crearse el directorio padre, si el pool no
/// puede construirse, o si la migración inicial falla.
pub fn build_pool(path: &Path, scope: DbScope) -> Result<Pool<SqliteConnectionManager>> {
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("failed to create directory {:?}", parent))?;
        }
    }

    let manager = SqliteConnectionManager::file(path).with_init(pragma_init);
    let pool = Pool::builder()
        .max_size(POOL_MAX_SIZE)
        .build(manager)
        .with_context(|| format!("failed to build pool for {:?}", path))?;

    {
        let conn = pool
            .get()
            .with_context(|| format!("failed to acquire pool conn for migrations {:?}", path))?;
        migrations::run(&conn, scope)?;
    }

    Ok(pool)
}

/// Abre una conexión SQLite y aplica las migraciones pendientes para el `scope` indicado.
///
/// Usado por `pillbox exec` (proceso pasivo, una operación por ejecución).
pub fn open(path: &Path, scope: DbScope) -> Result<Connection> {
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("failed to create directory {:?}", parent))?;
        }
    }

    let conn =
        Connection::open(path).with_context(|| format!("failed to open DB at {:?}", path))?;

    configure(&conn)?;
    migrations::run(&conn, scope)?;

    Ok(conn)
}

/// Abre una conexión SQLite sin crear el fichero si no existe.
///
/// A diferencia de `open`, no llama a `create_dir_all` ni crea el archivo.
/// Devuelve error si el path no existe en disco.
///
/// `scope` lo decide el caller — no se infiere de la DB (ver arch decision).
pub fn open_existing(path: &Path, scope: DbScope) -> Result<Connection> {
    if !path.exists() {
        anyhow::bail!("DB not found at {:?}", path);
    }
    let conn =
        Connection::open(path).with_context(|| format!("failed to open DB at {:?}", path))?;
    configure(&conn)?;
    migrations::run(&conn, scope)?;
    Ok(conn)
}

/// Abre una conexión en memoria — útil para tests. Cada test elige scope coherente.
pub fn open_in_memory(scope: DbScope) -> Result<Connection> {
    let conn = Connection::open_in_memory()?;
    configure(&conn)?;
    migrations::run(&conn, scope)?;
    Ok(conn)
}

/// Aplica los PRAGMA de configuración a una conexión ya abierta.
///
/// Se llama antes de cualquier operación — los PRAGMA no persisten
/// entre conexiones en SQLite.
fn configure(conn: &Connection) -> Result<()> {
    conn.execute_batch(PRAGMA_SQL)
        .context("failed to configure SQLite PRAGMAs")?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn open_in_memory_applies_migrations_local() {
        let conn = open_in_memory(DbScope::Local).unwrap();
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM schema_migrations", [], |r| r.get(0))
            .unwrap();
        assert!(count > 0);
    }

    #[test]
    fn open_in_memory_applies_migrations_global() {
        let conn = open_in_memory(DbScope::Global).unwrap();
        // La DB global debe tener `capsules`.
        let exists: bool = conn
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name='capsules')",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert!(exists, "global scope must create capsules table");
    }

    #[test]
    fn open_creates_parent_dirs_local() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("sub").join("pillbox.db");
        let _conn = open(&path, DbScope::Local).unwrap();
        assert!(path.exists());
    }

    #[test]
    fn open_creates_parent_dirs_global() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("sub").join("global.db");
        let _conn = open(&path, DbScope::Global).unwrap();
        assert!(path.exists());
    }
}
