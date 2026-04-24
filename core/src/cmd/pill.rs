use anyhow::Result;
use owo_colors::OwoColorize;
use rust_i18n::t;

use crate::output;

pub fn cmd_pill_show(id: i64) -> Result<()> {
    use pillbox::db::{connection, store::pills};

    let db_path =
        pillbox::config::resolve_db_path().unwrap_or_else(pillbox::config::global_db_path);

    if !db_path.exists() {
        output::fmt::db_not_found();
        return Ok(());
    }

    let conn = connection::open(&db_path)?;
    match pills::read(&conn, id)? {
        Some(pill) => output::fmt::pill_detail(&pill),
        None => eprintln!("\n{} {}\n", "✗".red(), t!("pill.error.not_found", id = id)),
    }
    Ok(())
}
