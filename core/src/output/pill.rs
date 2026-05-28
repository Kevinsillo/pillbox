//! Pantallas y formato relacionado con pills.

use super::table;
use owo_colors::OwoColorize;
use pillbox::db::store::id_resolver::display_id;
use pillbox::domain::pill::Pill;
use rust_i18n::t;

/// Muestra el detalle completo de una pill: metadatos y contenido formateado.
pub fn pill_detail(pill: &Pill) {
    let short_id = format!("id: {}", display_id(&pill.id));
    let rx_short = display_id(&pill.prescription_id);
    let author_val = pill.author_name.as_deref().unwrap_or("-").to_string();
    let rows = vec![
        [t!("pill.detail.id").bold().to_string(), short_id],
        [
            t!("pill.detail.views").bold().to_string(),
            pill.views.to_string(),
        ],
        [
            t!("pill.detail.compound").bold().to_string(),
            pill.compound.clone(),
        ],
        [
            t!("pill.detail.title").bold().to_string(),
            pill.title.clone(),
        ],
        [
            t!("pill.detail.prescription").bold().to_string(),
            rx_short.cyan().to_string(),
        ],
        [
            t!("pill.detail.created").bold().to_string(),
            pill.created_at.clone(),
        ],
        [t!("pill.detail.author").bold().to_string(), author_val],
    ];
    println!("\n{}", table::dict(rows));
    println!();
    for line in pill.content.lines() {
        println!("  {}", line);
    }
    println!();
}
