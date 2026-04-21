use std::path::Path;

use anyhow::{Context, Result};
use rusqlite::Connection;

use crate::db::migrations;

/// Abre una conexión SQLite y aplica las migraciones pendientes.
///
/// Usado por `pillbox exec` (proceso pasivo, una operación por ejecución).
/// Para `pillbox serve` se usará un pool — pendiente Fase 3.
pub fn open(path: &Path) -> Result<Connection> {
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("no se pudo crear el directorio {:?}", parent))?;
        }
    }

    let conn =
        Connection::open(path).with_context(|| format!("no se pudo abrir la DB en {:?}", path))?;

    configure(&conn)?;
    migrations::run(&conn)?;

    Ok(conn)
}

/// Abre una conexión SQLite sin crear el fichero si no existe.
///
/// A diferencia de `open`, no llama a `create_dir_all` ni crea el archivo.
/// Devuelve error si el path no existe en disco.
pub fn open_existing(path: &Path) -> Result<Connection> {
    if !path.exists() {
        anyhow::bail!("DB not found at {:?}", path);
    }
    let conn =
        Connection::open(path).with_context(|| format!("no se pudo abrir la DB en {:?}", path))?;
    configure(&conn)?;
    migrations::run(&conn)?;
    Ok(conn)
}

/// Abre una conexión en memoria — útil para tests.
pub fn open_in_memory() -> Result<Connection> {
    let conn = Connection::open_in_memory()?;
    configure(&conn)?;
    migrations::run(&conn)?;
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
    .context("error al configurar PRAGMA")?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn open_in_memory_applies_migrations() {
        let conn = open_in_memory().unwrap();
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM pill_compounds", [], |r| r.get(0))
            .unwrap();
        assert!(count > 0);
    }

    #[test]
    fn open_creates_parent_dirs() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("sub").join("pillbox.db");
        let _conn = open(&path).unwrap();
        assert!(path.exists());
    }
}
