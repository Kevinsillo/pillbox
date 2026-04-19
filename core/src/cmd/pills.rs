use anyhow::Result;
use rust_i18n::t;

use crate::output;

use super::shared::find_current_bottle;

pub fn cmd_pills_list() -> Result<()> {
    use pillbox::db::{connection, store::bottles};

    let global_path = pillbox::config::global_db_path();
    if !global_path.exists() {
        output::fmt::db_not_found();
        return Ok(());
    }

    let global_conn = connection::open(&global_path)?;
    let bottle = match find_current_bottle() {
        Ok(b) => b,
        Err(_) => {
            println!("{}\n", t!("bottle.error.not_found_short"));
            let all = bottles::list(&global_conn)?;
            output::fmt::bottles_list(&all);
            return Ok(());
        }
    };

    let conn = pillbox::db::connection::open(
        &pillbox::config::resolve_db_path().unwrap_or_else(|| global_path.clone()),
    )?;

    let mut stmt = conn.prepare(
        "SELECT p.id, p.compound, p.title, p.created_at
         FROM pills p
         JOIN prescriptions rx ON rx.id = p.prescription_id
         WHERE rx.bottle_id = ?1 AND p.deleted_at IS NULL
         ORDER BY p.created_at DESC",
    )?;

    let pills: Vec<(i64, String, String, String)> = stmt
        .query_map(rusqlite::params![bottle.id], |r| {
            Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?))
        })?
        .collect::<rusqlite::Result<_>>()?;

    output::fmt::pills_list(&bottle.name, &pills);
    Ok(())
}
