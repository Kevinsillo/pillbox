use anyhow::Result;
use rust_i18n::t;

use crate::output;

pub fn cmd_capsule_show(id: i64) -> Result<()> {
    use pillbox::db::{connection, store::capsules};

    let global_path = pillbox::config::global_db_path();
    if !global_path.exists() {
        output::fmt::db_not_found();
        return Ok(());
    }

    let conn = connection::open(&global_path)?;
    match capsules::read(&conn, id)? {
        Some(capsule) => output::fmt::capsule_detail(&capsule),
        None => eprintln!(
            "\n{} {}\n",
            "✗".red(),
            t!("capsule.error.not_found", id = id)
        ),
    }
    Ok(())
}

use owo_colors::OwoColorize;
