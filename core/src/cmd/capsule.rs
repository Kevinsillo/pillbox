//! Subcomandos `pillbox capsule *`.

use anyhow::Result;
use owo_colors::OwoColorize;
use rust_i18n::t;

use crate::output;

/// Muestra el detalle de una capsule por su UUID, incluyendo archivadas.
pub fn cmd_capsule_show(id: &str) -> Result<()> {
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

/// Lista las capsules globales (activas y archivadas) hasta los límites indicados.
///
/// Hace dos consultas separadas: activas (cap por `limit`) y archivadas
/// (cap por `archived_limit`). Una tercera consulta `COUNT(*)` sobre archivadas
/// permite mostrar el trailer `... N más archivados` con el número exacto.
pub fn cmd_capsule_list(limit: u32, archived_limit: u32) -> Result<()> {
    use pillbox::db::{connection, store::capsules, store::ListFilter};

    let global_path = pillbox::config::global_db_path();
    if !global_path.exists() {
        output::fmt::db_not_found();
        return Ok(());
    }

    let conn = connection::open(&global_path)?;
    let total = capsules::count(&conn)?;
    let active = capsules::list(&conn, Some(limit), None, ListFilter::Active)?;
    let (archived, archived_total) = if archived_limit == 0 {
        (Vec::new(), 0)
    } else {
        let rows = capsules::list(&conn, Some(archived_limit), None, ListFilter::Archived)?;
        let total_archived = capsules::count_archived(&conn)?;
        (rows, total_archived)
    };
    output::fmt::capsules_list(&active, &archived, archived_limit, archived_total, total);
    Ok(())
}
