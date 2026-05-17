use std::path::Path;

use anyhow::{Context, Result};
use rusqlite::Connection;

use crate::db::{migrations, DbScope};

/// Abre una conexión SQLite y aplica las migraciones pendientes para el `scope` indicado.
///
/// Usado por `pillbox exec` (proceso pasivo, una operación por ejecución).
/// Para `pillbox serve` se usará un pool — pendiente Fase 3.
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
    conn.execute_batch(
        "PRAGMA journal_mode  = WAL;
         PRAGMA busy_timeout  = 5000;
         PRAGMA synchronous   = NORMAL;
         PRAGMA foreign_keys  = ON;
         PRAGMA auto_vacuum   = INCREMENTAL;",
    )
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
