//! Pantallas y formato relacionado con capsules.

use super::{table, truncate};
use owo_colors::OwoColorize;
use pillbox::db::store::id_resolver::display_id;
use pillbox::domain::capsule::Capsule;
use rust_i18n::t;

/// Muestra la lista de capsules, separando activas de archivadas.
///
/// Caller pre-divide los registros entre `active` y `archived`. La función
/// aplica el cap visible (`archived_limit`) sobre los archivados y muestra
/// el trailer `... N más archivados` con `N = archived_total - archived_limit`
/// cuando el total exceda el cap. Si `archived_limit == 0`, oculta toda la
/// sección de archivadas.
pub fn capsules_list(
    active: &[Capsule],
    archived: &[Capsule],
    archived_limit: u32,
    archived_total: u32,
    total: u32,
) {
    let count = active.len();
    println!("\nCapsules    {}", count.to_string().bold());

    let archived_cap = archived_limit as usize;
    let archived_shown: Vec<&Capsule> = if archived_cap == 0 {
        Vec::new()
    } else {
        archived.iter().take(archived_cap).collect()
    };

    if active.is_empty() && archived_shown.is_empty() {
        println!("\n  {}\n", t!("capsules.none").dimmed());
        return;
    }

    if !active.is_empty() {
        println!();
        let rows = active
            .iter()
            .map(|c| {
                vec![
                    display_id(&c.id),
                    c.views.to_string().dimmed().to_string(),
                    truncate(&c.compound, 14),
                    truncate(&c.title, 50),
                ]
            })
            .collect();
        print!(
            "{}",
            table::plain_list(
                &[
                    t!("capsules.list.col.num").as_ref(),
                    "👁",
                    t!("capsules.list.col.compound").as_ref(),
                    t!("capsules.list.col.title").as_ref(),
                ],
                rows,
            )
        );
        let active_count = active.len() as u32;
        let hidden = total.saturating_sub(active_count + archived_total);
        if hidden > 0 {
            println!("   … {} más", hidden);
        } else {
            println!();
        }
    }

    if !archived_shown.is_empty() {
        let n = archived_shown.len();
        println!(
            "\n  {}\n",
            t!("capsules.list.archived_section", count = n).dimmed()
        );
        let rows = archived_shown
            .iter()
            .map(|c| {
                let archived_date_raw = c
                    .deleted_at
                    .as_deref()
                    .and_then(|d| d.get(..10))
                    .unwrap_or("—");
                let archived_date = archived_date_raw.dimmed().to_string();
                vec![
                    display_id(&c.id).dimmed().to_string(),
                    c.views.to_string().dimmed().to_string(),
                    truncate(&c.compound, 14).dimmed().to_string(),
                    truncate(&c.title, 46).dimmed().to_string(),
                    archived_date,
                ]
            })
            .collect();
        print!(
            "{}",
            table::plain_list(
                &[
                    t!("capsules.list.col.num").as_ref(),
                    "👁",
                    t!("capsules.list.col.compound").as_ref(),
                    t!("capsules.list.col.title").as_ref(),
                    t!("capsules.list.col.archived_at").as_ref(),
                ],
                rows,
            )
        );
        let archived_hidden = archived_total.saturating_sub(archived_cap as u32);
        if archived_hidden > 0 {
            println!(
                "   {}",
                t!("capsules.list.more_archived", count = archived_hidden).dimmed()
            );
        }
        println!();
    } else {
        println!();
    }
}

/// Muestra el detalle completo de una capsule: metadatos y contenido formateado.
pub fn capsule_detail(capsule: &Capsule) {
    let short_id = format!("id: {}", display_id(&capsule.id));
    let updated = if capsule.updated_at != capsule.created_at {
        capsule.updated_at.dimmed().to_string()
    } else {
        "—".dimmed().to_string()
    };
    let rows = vec![
        [t!("capsule.detail.id").bold().to_string(), short_id],
        [
            t!("capsule.detail.views").bold().to_string(),
            capsule.views.to_string(),
        ],
        [
            t!("capsule.detail.compound").bold().to_string(),
            capsule.compound.clone(),
        ],
        [
            t!("capsule.detail.title").bold().to_string(),
            capsule.title.clone(),
        ],
        [
            t!("capsule.detail.created").bold().to_string(),
            capsule.created_at.clone(),
        ],
        [t!("capsule.detail.updated").bold().to_string(), updated],
    ];
    println!("\n{}", table::dict(rows));
    println!();
    for line in capsule.content.lines() {
        println!("  {}", line);
    }
    println!();
}
