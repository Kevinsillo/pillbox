use anyhow::Result;
use indicatif::{ProgressBar, ProgressStyle};
use rust_i18n::t;
use std::time::Duration;

pub fn open_resolved_db() -> Result<(rusqlite::Connection, std::path::PathBuf)> {
    let path = pillbox::config::resolve_db_path()
        .ok_or_else(|| anyhow::anyhow!("{}", t!("db.open_not_found")))?;
    let conn = pillbox::db::connection::open(&path)?;
    Ok((conn, path))
}

pub fn find_current_bottle() -> Result<pillbox::domain::bottle::Bottle> {
    use pillbox::db::{connection, store::bottles};
    let global_path = pillbox::config::global_db_path();
    let conn = connection::open(&global_path)?;
    let current = std::env::current_dir()?;
    bottles::list(&conn)?
        .into_iter()
        .filter(|b| current.starts_with(&b.directory))
        .max_by_key(|b| b.directory.len())
        .ok_or_else(|| anyhow::anyhow!("{}", t!("bottle.error.not_found")))
}

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
