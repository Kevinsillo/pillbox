//! Subcomandos `pillbox capsule *`.

use anyhow::Result;
use owo_colors::OwoColorize;
use rust_i18n::t;

use crate::output;

/// Muestra el detalle de una capsule por su ID numérico, incluyendo archivadas.
pub fn cmd_capsule_show(id: i64) -> Result<()> {
    use pillbox::db::{connection, store::capsules};

    let global_path = pillbox::config::global_db_path();
    if !global_path.exists() {
        output::fmt::db_not_found();
        return Ok(());
    }

    let conn = connection::open(&global_path)?;
    match capsules::read_any(&conn, id)? {
        Some(capsule) => output::fmt::capsule_detail(&capsule),
        None => eprintln!(
            "\n{} {}\n",
            "✗".red(),
            t!("capsule.error.not_found", id = id)
        ),
    }
    Ok(())
}

/// Lista las capsules globales (activas y archivadas) hasta el límite indicado.
pub fn cmd_capsule_list(limit: u32) -> Result<()> {
    use pillbox::db::{connection, store::capsules};

    let global_path = pillbox::config::global_db_path();
    if !global_path.exists() {
        output::fmt::db_not_found();
        return Ok(());
    }

    let conn = connection::open(&global_path)?;
    let total = capsules::count(&conn)?;
    let list = capsules::list(&conn, Some(limit), None)?;
    output::fmt::capsules_list(&list, total);
    Ok(())
}
