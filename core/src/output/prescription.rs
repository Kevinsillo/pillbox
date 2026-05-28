//! Pantallas y formato relacionado con prescripciones.

use super::layout::{print_a, print_b};
use super::{table, truncate};
use owo_colors::OwoColorize;
use pillbox::db::store::id_resolver::display_id;
use pillbox::domain::prescription::Prescription;
use rust_i18n::t;

/// Muestra la lista de prescripciones de un bottle.
///
/// Caller pre-divide los registros entre `active` y `archived`. La función
/// aplica el cap visible (`archived_limit`) sobre los archivados y muestra
/// el trailer `... N más archivados` con `N = archived_total - archived_limit`
/// cuando el total exceda el cap. Si `archived_limit == 0`, oculta toda la
/// sección de archivadas (sin header, sin filas, sin trailer).
pub fn prescriptions_list(
    bottle_name: &str,
    db_path: &str,
    active: &[Prescription],
    archived: &[Prescription],
    archived_limit: u32,
    archived_total: u32,
    total: u32,
) {
    let count = active.len();
    println!(
        "\nPrescriptions  {}  {}    {}",
        "·".dimmed(),
        bottle_name.dimmed(),
        count.to_string().bold()
    );
    println!("  {}", db_path.dimmed());

    let archived_cap = archived_limit as usize;
    let archived_shown: Vec<&Prescription> = if archived_cap == 0 {
        Vec::new()
    } else {
        archived.iter().take(archived_cap).collect()
    };

    if active.is_empty() && archived_shown.is_empty() {
        println!("\n  {}\n", t!("prescriptions.none").dimmed());
        return;
    }

    if !active.is_empty() {
        println!();
        let rows = active
            .iter()
            .map(|rx| {
                let short_id = &display_id(&rx.id);
                let estado = if rx.ended_at.is_some() {
                    t!("prescriptions.state.closed").dimmed().to_string()
                } else {
                    t!("prescriptions.state.open").green().to_string()
                };
                let author = match &rx.author_name {
                    Some(name) => truncate(name, 18),
                    None => "-".dimmed().to_string(),
                };
                vec![
                    short_id.to_string(),
                    rx.views.to_string().dimmed().to_string(),
                    truncate(&rx.title, 40),
                    estado,
                    author,
                ]
            })
            .collect();
        print!(
            "{}",
            table::plain_list(
                &[
                    t!("prescriptions.list.col.id").as_ref(),
                    "👁",
                    t!("prescriptions.list.col.title").as_ref(),
                    t!("prescriptions.list.col.state").as_ref(),
                    t!("prescriptions.list.col.author").as_ref(),
                ],
                rows
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
            t!("prescriptions.list.archived_section", count = n).dimmed()
        );
        let rows = archived_shown
            .iter()
            .map(|rx| {
                let short_id = &display_id(&rx.id);
                let estado = if rx.ended_at.is_some() {
                    t!("prescriptions.state.closed").dimmed().to_string()
                } else {
                    t!("prescriptions.state.open").dimmed().to_string()
                };
                let archived_date_raw = rx
                    .deleted_at
                    .as_deref()
                    .and_then(|d| d.get(..10))
                    .unwrap_or("—");
                let archived_date = archived_date_raw.dimmed().to_string();
                let author = match &rx.author_name {
                    Some(name) => truncate(name, 18).dimmed().to_string(),
                    None => "-".dimmed().to_string(),
                };
                vec![
                    short_id.dimmed().to_string(),
                    rx.views.to_string().dimmed().to_string(),
                    truncate(&rx.title, 36).dimmed().to_string(),
                    estado,
                    author,
                    archived_date,
                ]
            })
            .collect();
        print!(
            "{}",
            table::plain_list(
                &[
                    t!("prescriptions.list.col.id").as_ref(),
                    "👁",
                    t!("prescriptions.list.col.title").as_ref(),
                    t!("prescriptions.list.col.state").as_ref(),
                    t!("prescriptions.list.col.author").as_ref(),
                    t!("prescriptions.list.col.archived_at").as_ref(),
                ],
                rows
            )
        );
        let archived_hidden = archived_total.saturating_sub(archived_cap as u32);
        if archived_hidden > 0 {
            println!(
                "   {}",
                t!("prescriptions.list.more_archived", count = archived_hidden).dimmed()
            );
        }
        println!();
    } else {
        println!();
    }
}

/// Confirma en pantalla la apertura de una prescripción nueva.
pub fn prescription_opened(id: &str, title: &str) {
    let short_id = &display_id(id);
    print_a(
        &t!("prescriptions.msg.opened"),
        &[("title", title), ("id", short_id)],
    );
}

/// Confirma en pantalla el cierre de una prescripción.
pub fn prescription_closed(_title: &str) {
    print_b(&t!("prescriptions.msg.closed"));
}

/// Confirma en pantalla la reapertura de una prescripción.
pub fn prescription_reopened(id: &str, title: &str) {
    let short_id = &display_id(id);
    print_a(
        &t!("prescriptions.msg.reopened"),
        &[("title", title), ("id", short_id)],
    );
}

/// Muestra el detalle completo de una prescripción con sus pills activas y archivadas.
///
/// Las pills activas se limitan a `limit`. Las archivadas se limitan a
/// `archived_limit`; cuando `archived_total > archived_limit` se imprime
/// el trailer `... N más archivados` con `N = archived_total - archived_limit`.
/// Si `archived_limit == 0`, oculta la sección de archivadas entera.
pub fn prescription_show(
    rx: &Prescription,
    pills: &[pillbox::domain::pill::Pill],
    limit: u32,
    archived_limit: u32,
    archived_total: u32,
) {
    let estado = if rx.ended_at.is_some() {
        t!("prescriptions.state.closed").dimmed().to_string()
    } else {
        t!("prescriptions.state.open").green().to_string()
    };
    let short_id = &display_id(&rx.id);
    let author_val = rx.author_name.as_deref().unwrap_or("-").to_string();
    let rows = vec![
        [
            t!("prescription.show.id").bold().to_string(),
            short_id.cyan().to_string(),
        ],
        [
            t!("prescription.show.views").bold().to_string(),
            rx.views.to_string(),
        ],
        [
            t!("prescription.show.title").bold().to_string(),
            rx.title.clone(),
        ],
        [t!("prescription.show.state").bold().to_string(), estado],
        [
            t!("prescription.show.started").bold().to_string(),
            rx.started_at.clone(),
        ],
        [
            t!("prescription.show.author").bold().to_string(),
            author_val,
        ],
    ];
    println!("\n{}", table::dict(rows));

    let (active_pills, archived_pills): (
        Vec<&pillbox::domain::pill::Pill>,
        Vec<&pillbox::domain::pill::Pill>,
    ) = pills.iter().partition(|p| p.deleted_at.is_none());

    let total_active = active_pills.len();
    let shown_active: Vec<&pillbox::domain::pill::Pill> =
        active_pills.iter().copied().take(limit as usize).collect();
    let hidden = total_active.saturating_sub(shown_active.len());

    let archived_cap = archived_limit as usize;
    let archived_shown: Vec<&pillbox::domain::pill::Pill> = if archived_cap == 0 {
        Vec::new()
    } else {
        archived_pills.iter().copied().take(archived_cap).collect()
    };

    println!(
        "\nPills  {}  {}    {}",
        "·".dimmed(),
        short_id.dimmed(),
        total_active.to_string().bold()
    );

    if active_pills.is_empty() && archived_shown.is_empty() {
        println!("\n  {}\n", t!("pills.none").dimmed());
        return;
    }

    if !shown_active.is_empty() {
        println!();
        let table_rows = shown_active
            .iter()
            .map(|p| {
                vec![
                    display_id(&p.id),
                    p.views.to_string().dimmed().to_string(),
                    truncate(&p.compound, 16),
                    truncate(&p.title, 50),
                    p.author_name
                        .as_deref()
                        .map(|n| truncate(n, 18))
                        .unwrap_or_else(|| "-".dimmed().to_string()),
                ]
            })
            .collect();
        print!(
            "{}",
            table::plain_list(
                &[
                    t!("pills.list.col.num").as_ref(),
                    "👁",
                    t!("pills.list.col.compound").as_ref(),
                    t!("pills.list.col.title").as_ref(),
                    t!("pills.list.col.author").as_ref(),
                ],
                table_rows,
            )
        );
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
            t!("prescription.pills.archived_section", count = n).dimmed()
        );
        let table_rows = archived_shown
            .iter()
            .map(|p| {
                vec![
                    p.id.to_string().dimmed().to_string(),
                    p.views.to_string().dimmed().to_string(),
                    truncate(&p.compound, 16).dimmed().to_string(),
                    truncate(&p.title, 50).dimmed().to_string(),
                    p.author_name
                        .as_deref()
                        .map(|n| truncate(n, 18).dimmed().to_string())
                        .unwrap_or_else(|| "-".dimmed().to_string()),
                ]
            })
            .collect();
        print!(
            "{}",
            table::plain_list(
                &[
                    t!("pills.list.col.num").as_ref(),
                    "👁",
                    t!("pills.list.col.compound").as_ref(),
                    t!("pills.list.col.title").as_ref(),
                    t!("pills.list.col.author").as_ref(),
                ],
                table_rows,
            )
        );
        let archived_hidden = archived_total.saturating_sub(archived_cap as u32);
        if archived_hidden > 0 {
            println!(
                "   {}",
                t!("prescription.pills.more_archived", count = archived_hidden).dimmed()
            );
        }
        println!();
    } else {
        println!();
    }
}
