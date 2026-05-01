//! Subcomandos `pillbox prescription *`.

use anyhow::Result;
use owo_colors::OwoColorize;
use rust_i18n::t;

use crate::output;

use super::shared::find_current_bottle;
use super::shared::open_resolved_db;

/// Abre una nueva prescription para el bottle del directorio actual.
pub fn cmd_prescription_open(title: String) -> Result<()> {
    use pillbox::db::store::prescriptions;
    use pillbox::domain::prescription::NewPrescription;
    use pillbox::error::PillboxError;

    let (mut conn, _) = open_resolved_db()?;
    let bottle = find_current_bottle()?;

    match prescriptions::open(
        &mut conn,
        &NewPrescription {
            bottle_id: bottle.id,
            title,
            author_name: None,
            author_email: None,
        },
    ) {
        Ok(rx) => output::fmt::prescription_opened(&rx.id, &rx.title),
        Err(e) => {
            if let Some(PillboxError::PrescriptionAlreadyOpen {
                ref id,
                ref title,
                pill_count,
                ..
            }) = e.downcast_ref::<PillboxError>()
            {
                anyhow::bail!(
                    "{}",
                    t!(
                        "prescriptions.error.already_open",
                        title = title,
                        pills = pill_count,
                        id = &id[..id.len().min(8)]
                    )
                );
            }
            return Err(e);
        }
    }
    Ok(())
}

/// Lista las prescriptions del bottle actual (más recientes primero).
pub fn cmd_prescription_list(limit: u32) -> Result<()> {
    use pillbox::db::store::prescriptions;

    let (conn, db_path) = open_resolved_db()?;
    let bottle = find_current_bottle()?;
    let total = prescriptions::count_by_bottle(&conn, &bottle.id)?;
    let rxs = prescriptions::list_by_bottle(&conn, &bottle.id, limit)?;

    output::fmt::prescriptions_list(&bottle.name, &db_path.display().to_string(), &rxs, total);
    Ok(())
}

/// Muestra el detalle de una prescription (incluyendo descartadas) y sus pills.
pub fn cmd_prescription_show(id: String, limit: u32) -> Result<()> {
    use pillbox::db::store::{pills, prescriptions};

    let (conn, _) = open_resolved_db()?;

    let rx = match prescriptions::read_any(&conn, &id)? {
        Some(rx) => rx,
        None => {
            eprintln!(
                "\n{} {}\n",
                "✗".red(),
                t!("prescriptions.error.not_found", id = &id[..id.len().min(8)])
            );
            return Ok(());
        }
    };

    let pill_list = pills::list_by_prescription(&conn, &rx.id)?;
    output::fmt::prescription_show(&rx, &pill_list, limit);
    Ok(())
}

/// Cierra la prescription abierta del bottle actual.
pub fn cmd_prescription_close() -> Result<()> {
    use pillbox::db::store::prescriptions;

    let (mut conn, _) = open_resolved_db()?;
    let bottle = find_current_bottle()?;

    let open_rx: Option<(String, String)> = conn
        .query_row(
            "SELECT id, title FROM prescriptions
             WHERE bottle_id = ?1 AND ended_at IS NULL AND deleted_at IS NULL
             LIMIT 1",
            rusqlite::params![bottle.id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .ok();

    let (rx_id, rx_title) = open_rx.ok_or_else(|| {
        anyhow::anyhow!(
            "{}",
            t!("prescriptions.error.none_open", name = bottle.name)
        )
    })?;

    prescriptions::close(&mut conn, &rx_id)?;
    output::fmt::prescription_closed(&rx_title);
    Ok(())
}
