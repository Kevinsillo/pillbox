//! Limpieza de ficheros de DB local cuando dejan de tener bottles asociados.
//!
//! Provee helpers para borrar el `.db` y sus sidecars (`-wal`, `-shm`) y un
//! wrapper que sólo borra si el `registered_bottles` ya no tiene entradas
//! apuntando a esa ruta, con un guard para nunca tocar la DB global.

use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use rusqlite::{params, Connection};

use crate::config;

/// Borra el fichero `.db` indicado junto con sus sidecars `-wal` y `-shm`.
///
/// Los sidecars se construyen anexando `-wal` y `-shm` al *filename* del path
/// (no como entradas de directorio). Errores `NotFound` se silencian tanto en
/// el `.db` principal como en los sidecars; cualquier otro error se propaga
/// con contexto.
pub fn remove_local_db_files(path: &Path) -> Result<()> {
    remove_if_exists(path)
        .with_context(|| format!("failed to remove db file: {}", path.display()))?;

    for suffix in ["-wal", "-shm"] {
        let sidecar = sidecar_path(path, suffix);
        remove_if_exists(&sidecar)
            .with_context(|| format!("failed to remove sidecar: {}", sidecar.display()))?;
    }

    Ok(())
}

/// Si `registered_bottles` ya no contiene entradas con `db_path == path`,
/// borra los ficheros físicos y devuelve `Ok(true)`. Si quedan entradas o el
/// path coincide (canonical) con la DB global, devuelve `Ok(false)` sin tocar
/// nada.
pub fn cleanup_if_last_bottle(conn: &Connection, path: &Path) -> Result<bool> {
    // Guard de locality: nunca borrar la DB global.
    let target_canon = canonicalize_or_self(path);
    let global_canon = canonicalize_or_self(&config::global_db_path());
    if target_canon == global_canon {
        return Ok(false);
    }

    let path_str = path.to_string_lossy();
    let count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM registered_bottles WHERE db_path = ?1",
            params![path_str.as_ref()],
            |row| row.get(0),
        )
        .context("failed to count registered bottles for db_path")?;

    if count == 0 {
        remove_local_db_files(path)?;
        Ok(true)
    } else {
        Ok(false)
    }
}

/// Construye la ruta de un sidecar anexando `suffix` al nombre del fichero.
fn sidecar_path(path: &Path, suffix: &str) -> PathBuf {
    let mut file_name = path
        .file_name()
        .map(|s| s.to_os_string())
        .unwrap_or_default();
    file_name.push(suffix);
    match path.parent() {
        Some(parent) if !parent.as_os_str().is_empty() => parent.join(file_name),
        _ => PathBuf::from(file_name),
    }
}

/// Borra `path` silenciando `NotFound`; propaga cualquier otro error.
fn remove_if_exists(path: &Path) -> std::io::Result<()> {
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == ErrorKind::NotFound => Ok(()),
        Err(e) => Err(e),
    }
}

/// Devuelve `canonicalize(path)` si existe; en caso contrario devuelve el
/// `path` tal cual (no se puede canonicalizar lo inexistente).
fn canonicalize_or_self(path: &Path) -> PathBuf {
    fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
}
