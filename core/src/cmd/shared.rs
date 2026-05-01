//! Utilidades compartidas entre subcomandos del CLI.

use anyhow::Result;
use indicatif::{ProgressBar, ProgressStyle};
use rust_i18n::t;
use std::time::Duration;

/// Abre la DB resuelta por [`pillbox::config::resolve_db_path`] y devuelve la conexión y su ruta.
///
/// # Errors
///
/// Devuelve error si no se encuentra ninguna DB (global ni local).
pub fn open_resolved_db() -> Result<(rusqlite::Connection, std::path::PathBuf)> {
    let path = pillbox::config::resolve_db_path()
        .ok_or_else(|| anyhow::anyhow!("{}", t!("db.open_not_found")))?;
    let abs = path
        .canonicalize()
        .unwrap_or_else(|_| std::env::current_dir().unwrap_or_default().join(&path));
    let conn = pillbox::db::connection::open(&abs)?;
    Ok((conn, abs))
}

/// Encuentra el bottle asociado al directorio de trabajo actual.
///
/// Busca primero en la DB global (scope global) y luego en las DBs locales
/// registradas (scope local). Si el `cwd` está dentro del directorio de más
/// de un bottle, elige el más específico (match de prefijo más largo).
///
/// # Errors
///
/// Devuelve error si no se encuentra ningún bottle para el directorio actual.
pub fn find_current_bottle() -> Result<pillbox::domain::bottle::Bottle> {
    use pillbox::db::{
        connection,
        store::{bottles, registered_bottles},
    };
    let global_path = pillbox::config::global_db_path();
    let global_conn = connection::open(&global_path)?;
    let current = std::env::current_dir()?;

    let mut best: Option<(usize, pillbox::domain::bottle::Bottle)> = None;

    // Global-scope bottles: stored directly in the global DB bottles table
    for bottle in bottles::list(&global_conn).unwrap_or_default() {
        if current.starts_with(&bottle.directory) {
            let len = bottle.directory.len();
            if best.as_ref().map_or(true, |(l, _)| len > *l) {
                best = Some((len, bottle));
            }
        }
    }

    // Local-scope bottles: stored in local DBs, referenced in registered_bottles
    for reg in registered_bottles::list(&global_conn).unwrap_or_default() {
        let db_path = std::path::Path::new(&reg.db_path);
        if db_path == global_path.as_path() || !db_path.exists() {
            continue;
        }
        let derived_dir = db_path.parent().and_then(|p| p.parent());
        if let Some(dir) = derived_dir {
            if current.starts_with(dir) {
                let len = dir.to_string_lossy().len();
                if best.as_ref().map_or(true, |(l, _)| len > *l) {
                    if let Ok(local_conn) = connection::open(db_path) {
                        if let Ok(Some(bottle)) = bottles::find_by_id(&local_conn, &reg.bottle_id) {
                            best = Some((len, bottle));
                        }
                    }
                }
            }
        }
    }

    best.map(|(_, b)| b)
        .ok_or_else(|| anyhow::anyhow!("{}", t!("bottle.error.not_found")))
}

/// Crea un spinner de progreso con tick automático cada 80 ms.
pub fn spinner(msg: impl Into<std::borrow::Cow<'static, str>>) -> ProgressBar {
    let pb = ProgressBar::new_spinner();
    pb.enable_steady_tick(Duration::from_millis(80));
    pb.set_style(
        ProgressStyle::default_spinner()
            .tick_strings(&["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"])
            .template("{spinner} {msg}")
            .unwrap(),
    );
    pb.set_message(msg);
    pb
}
