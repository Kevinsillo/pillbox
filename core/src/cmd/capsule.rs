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
    use pillbox::domain::PaginationParams;

    let global_path = pillbox::config::global_db_path();
    if !global_path.exists() {
        output::fmt::db_not_found();
        return Ok(());
    }

    let conn = connection::open(&global_path)?;
    let total = capsules::count(&conn)?;
    let pagination = PaginationParams { page: 1, page_size: limit.max(1).min(100) };
    let active = capsules::list(&conn, ListFilter::Active, None, &pagination)?.items;
    let archived_pagination = PaginationParams { page: 1, page_size: archived_limit.max(1).min(100) };
    let archived = capsules::list(&conn, ListFilter::Archived, None, &archived_pagination)?.items;
    let archived_total = capsules::count_archived(&conn)?;
    output::fmt::capsules_list(&active, &archived, archived_limit, archived_total, total);
    Ok(())
}
