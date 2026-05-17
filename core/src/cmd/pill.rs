//! Subcomandos `pillbox pill *`.

use anyhow::Result;
use owo_colors::OwoColorize;
use rust_i18n::t;

use crate::output;

/// Muestra el detalle de una pill por su UUID, incluyendo descartadas.
///
/// Intenta primero la DB resuelta por contexto (local o global); si no existe,
/// usa la DB global como fallback.
pub fn cmd_pill_show(id: &str) -> Result<()> {
    use pillbox::db::{connection, store::pills, DbScope};

    let db_path =
        pillbox::config::resolve_db_path_from_env().unwrap_or_else(pillbox::config::global_db_path);

    if !db_path.exists() {
        output::fmt::db_not_found();
        return Ok(());
    }

    // Inferir scope por path: Global si coincide con global_db_path, Local en caso contrario.
    let scope = if db_path == pillbox::config::global_db_path() {
        DbScope::Global
    } else {
        DbScope::Local
    };
    let conn = connection::open(&db_path, scope)?;
    match pills::read_any(&conn, id)? {
        Some(pill) => output::fmt::pill_detail(&pill),
        None => eprintln!("\n{} {}\n", "✗".red(), t!("pill.error.not_found", id = id)),
    }
    Ok(())
}
