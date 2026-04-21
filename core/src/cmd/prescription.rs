use anyhow::Result;
use rust_i18n::t;

use crate::output;

use super::shared::{find_current_bottle, open_resolved_db};

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

pub fn cmd_prescription_list(limit: u32) -> Result<()> {
    use pillbox::db::store::prescriptions;

    let (conn, _) = open_resolved_db()?;
    let bottle = find_current_bottle()?;
    let rxs = prescriptions::list_by_bottle(&conn, &bottle.id, limit)?;

    output::fmt::prescriptions_list(&bottle.name, &rxs, limit);
    Ok(())
}

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
