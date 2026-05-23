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

    let (mut conn, _) = open_resolved_db()?;
    let bottle = find_current_bottle()?;

    let rx = prescriptions::open(
        &mut conn,
        &NewPrescription {
            bottle_id: bottle.id,
            title,
            author_name: None,
            author_email: None,
        },
    )?;
    output::fmt::prescription_opened(&rx.id, &rx.title);
    Ok(())
}

/// Lista las prescriptions del bottle actual (más recientes primero).
///
/// Hace dos consultas separadas: activas (cap por `limit`) y archivadas
/// (cap por `archived_limit`). Una tercera consulta `COUNT(*)` sobre archivadas
/// permite mostrar el trailer `... N más archivados` con el número exacto.
pub fn cmd_prescription_list(limit: u32, archived_limit: u32) -> Result<()> {
    use pillbox::db::store::{prescriptions, ListFilter};
    use pillbox::domain::PaginationParams;

    let (conn, db_path) = open_resolved_db()?;
    let bottle = find_current_bottle()?;
    let total = prescriptions::count_by_bottle(&conn, &bottle.id)?;
    let pagination = PaginationParams {
        page: 1,
        page_size: limit.clamp(1, 100),
    };
    let active =
        prescriptions::list_by_bottle(&conn, &bottle.id, ListFilter::Active, &pagination)?.items;
    let archived_pagination = PaginationParams {
        page: 1,
        page_size: archived_limit.clamp(1, 100),
    };
    let archived = prescriptions::list_by_bottle(
        &conn,
        &bottle.id,
        ListFilter::Archived,
        &archived_pagination,
    )?
    .items;
    let archived_total = prescriptions::count_archived_by_bottle(&conn, &bottle.id)?;

    output::fmt::prescriptions_list(
        &bottle.name,
        &db_path.display().to_string(),
        &active,
        &archived,
        archived_limit,
        archived_total,
        total,
    );
    Ok(())
}

/// Muestra el detalle de una prescription (incluyendo descartadas) y sus pills.
pub fn cmd_prescription_show(id: String, limit: u32, archived_limit: u32) -> Result<()> {
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

    let pill_list = pills::list_by_prescription(
        &conn,
        &rx.id,
        pillbox::db::store::ListFilter::All,
        &pillbox::domain::PaginationParams {
            page: 1,
            page_size: 100,
        },
    )?
    .items;
    let archived_total = prescriptions::count_archived_pills(&conn, &rx.id)?;
    output::fmt::prescription_show(&rx, &pill_list, limit, archived_limit, archived_total);
    Ok(())
}

/// Cierra una prescription del bottle actual.
///
/// Si `id` se provee, cierra esa rx. Si no:
/// - 0 abiertas → error `none_open`.
/// - 1 abierta → cierra automáticamente.
/// - ≥2 abiertas → error `multiple_open` con la lista de candidatas.
pub fn cmd_prescription_close(id: Option<String>) -> Result<()> {
    use pillbox::db::store::id_resolver::display_id;
    use pillbox::db::store::prescriptions;

    let (mut conn, _) = open_resolved_db()?;
    let bottle = find_current_bottle()?;

    let rx_title = if let Some(id) = id {
        let rx = prescriptions::close(&mut conn, &id)?;
        rx.title
    } else {
        let mut stmt = conn.prepare(
            "SELECT id, title FROM prescriptions
             WHERE bottle_id = ?1 AND ended_at IS NULL AND deleted_at IS NULL
             ORDER BY started_at DESC",
        )?;
        let open_list: Vec<(String, String)> = stmt
            .query_map(rusqlite::params![bottle.id], |r| {
                Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?))
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        drop(stmt);

        match open_list.len() {
            0 => {
                anyhow::bail!(
                    "{}",
                    t!("prescriptions.error.none_open", name = bottle.name)
                );
            }
            1 => {
                let (rx_id, rx_title) = open_list.into_iter().next().unwrap();
                prescriptions::close(&mut conn, &rx_id)?;
                rx_title
            }
            _ => {
                let list = open_list
                    .iter()
                    .map(|(id, title)| format!("  - {}  {}", display_id(id), title))
                    .collect::<Vec<_>>()
                    .join("\n");
                anyhow::bail!("{}", t!("prescriptions.error.multiple_open", list = list));
            }
        }
    };

    output::fmt::prescription_closed(&rx_title);
    Ok(())
}

/// Reabre una prescription cerrada del bottle actual.
///
/// Idempotente: si la rx ya está abierta, devuelve éxito sin cambios.
pub fn cmd_prescription_reopen(id: String) -> Result<()> {
    use pillbox::db::store::prescriptions;
    use pillbox::error::PillboxError;

    let (mut conn, _) = open_resolved_db()?;

    match prescriptions::reopen(&mut conn, &id) {
        Ok(rx) => {
            output::fmt::prescription_reopened(&rx.id, &rx.title);
            Ok(())
        }
        Err(e) => {
            if let Some(PillboxError::PrescriptionNotFound { id }) =
                e.downcast_ref::<PillboxError>()
            {
                let short = &id[..id.len().min(8)];
                eprintln!(
                    "\n{} {}\n",
                    "✗".red(),
                    t!("prescriptions.error.not_found", id = short)
                );
                return Ok(());
            }
            Err(e)
        }
    }
}
