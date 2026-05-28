//! Pantallas y formato relacionado con bottles.

use super::layout::{print_a, print_b};
use super::{table, truncate};
use owo_colors::OwoColorize;
use pillbox::db::store::id_resolver::display_id;
use pillbox::domain::bottle::Bottle;
use rust_i18n::t;

/// Fila de datos para la tabla de listing de bottles registrados.
pub struct BottleListRow {
    pub name: String,
    pub directory: String,
    pub scope: String,
    pub linked: bool,
    pub is_active: bool,
    pub views: i64,
}

/// Muestra la lista de bottles registrados con su estado de enlace.
pub fn bottles_registered_list(rows: &[BottleListRow], total: u32) {
    let count = rows.len();
    println!("\nBottles    {}", count.to_string().bold());

    if rows.is_empty() {
        println!("\n  {}\n", t!("bottles.none").dimmed());
        return;
    }

    println!();
    let table_rows = rows
        .iter()
        .map(|r| {
            let estado = if !r.linked {
                "✗".red().to_string()
            } else if r.is_active {
                "●".green().to_string()
            } else {
                "○".dimmed().to_string()
            };

            let name_cell = if r.linked {
                r.name.clone()
            } else {
                r.name.dimmed().to_string()
            };

            let dir_cell = if r.linked {
                truncate(&r.directory, 50)
            } else {
                format!(
                    "{}  {}",
                    truncate(&r.directory, 40),
                    t!("bottles.unlinked").red()
                )
            };

            let scope_cell = r.scope.dimmed().to_string();

            let views_cell = r.views.to_string().dimmed().to_string();
            vec![estado, views_cell, name_cell, dir_cell, scope_cell]
        })
        .collect();

    print!(
        "{}",
        table::plain_list(
            &[
                " ",
                "👁",
                t!("bottles.list.col.name").as_ref(),
                t!("bottles.list.col.dir").as_ref(),
                t!("bottles.list.col.scope").as_ref(),
            ],
            table_rows,
        )
    );
    let hidden = total.saturating_sub(rows.len() as u32);
    if hidden > 0 {
        println!("   … {} más", hidden);
    } else {
        println!();
    }
    println!();
}

/// Muestra el estado actual de un bottle: nombre, scope, directorio, pills y prescripciones abiertas.
pub fn bottle_status(bottle: &Bottle, pill_count: i64, open_rxs: Vec<(String, String)>) {
    let rx_val = if open_rxs.is_empty() {
        t!("bottle.status.rx_none").dimmed().to_string()
    } else {
        open_rxs
            .iter()
            .map(|(id, title)| {
                t!("bottle.status.rx_open", title = title, id = &display_id(id)).to_string()
            })
            .collect::<Vec<_>>()
            .join("\n")
    };
    println!(
        "\n{}",
        table::dict(vec![
            [
                t!("bottle.status.labels.bottle").bold().to_string(),
                format!("{} — \"{}\"", bottle.name, bottle.display_name),
            ],
            [
                t!("bottle.status.labels.views").bold().to_string(),
                bottle.views.to_string(),
            ],
            [
                t!("bottle.status.labels.scope").bold().to_string(),
                bottle.scope.to_string(),
            ],
            [
                t!("bottle.status.labels.dir").bold().to_string(),
                bottle.directory.clone(),
            ],
        ])
    );
    println!(
        "\n{}\n",
        table::dict(vec![
            [
                t!("bottle.status.labels.pills").bold().to_string(),
                pill_count.to_string(),
            ],
            [t!("bottle.status.labels.rx").bold().to_string(), rx_val],
        ])
    );
}

/// Informa al usuario del inicio del proceso de inicialización de un bottle.
pub fn bottle_init_start(dir: &str) {
    println!("\n{}\n", t!("bottle.init.start", dir = dir));
}

/// Mensaje de error: fallo al registrar el bottle en la DB global durante init.
pub fn bottle_init_register_err(err: &str) {
    eprintln!("{}", t!("bottle.init.register_err", err = err));
}

/// Confirma la creación del bottle mostrando su slug, nombre visible y ruta de la DB.
pub fn bottle_init_created(name: &str, display_name: &str, db_path: &std::path::Path) {
    let db_str = db_path.display().to_string();
    let action = t!("bottle.init.created", name = name).to_string();
    let slug_label = t!("bottle.init.col.slug").to_string();
    let display_label = t!("bottle.init.col.display").to_string();
    let db_label = t!("bottle.init.col.db").to_string();
    print_a(
        &action,
        &[
            (&slug_label, name),
            (&display_label, display_name),
            (&db_label, &db_str),
        ],
    );
}

/// Confirma que `.pillbox/` fue añadido al `.gitignore` del proyecto.
pub fn bottle_init_gitignore() {
    print_b(&t!("bottle.init.gitignore.done"));
}

/// Imprime el mensaje de finalización del flujo de inicialización del bottle.
pub fn bottle_init_done() {
    println!("   {}  {}\n", "→".dimmed(), t!("bottle.init.done").dimmed());
}

/// Confirma que la DB local fue vinculada al registro global.
pub fn bottle_vinculate_done(name: &str, path: &str) {
    print_b(&t!("bottle.vinculate.done", name = name, path = path));
}

/// Mensaje de error: la DB local indicada no existe al intentar vincularla.
pub fn bottle_vinculate_db_not_found(path: &str) {
    eprintln!(
        "\n{}  {}\n",
        "✗".red().bold(),
        t!("bottle.vinculate.error_db_not_found", path = path)
    );
}

/// Mensaje de error: la DB local indicada apunta a la DB global (vínculo circular).
pub fn bottle_vinculate_circular() {
    eprintln!(
        "\n{}  {}\n",
        "✗".red().bold(),
        t!("bottle.vinculate.error_circular")
    );
}

/// Mensaje de error: la DB local no contiene ningún bottle para vincular.
pub fn bottle_vinculate_no_bottle(path: &str) {
    eprintln!(
        "\n{}  {}\n",
        "✗".red().bold(),
        t!("bottle.vinculate.error_no_bottle", path = path)
    );
}

/// Informa que la DB local ya estaba vinculada al registro global.
pub fn bottle_vinculate_already(name: &str, path: &str) {
    println!(
        "\n{} {}\n",
        "→".dimmed(),
        t!("bottle.vinculate.already_linked", name = name, path = path).dimmed()
    );
}

// ─── Bottle delete ────────────────────────────────────────────────────────────

/// Imprime el banner de confirmación con los datos del bottle a eliminar.
pub fn bottle_delete_panel(display_name: &str, slug: &str, db_path: &str) {
    let name_label = t!("bottle.delete.col.name");
    let slug_label = t!("bottle.delete.col.slug");
    let path_label = t!("bottle.delete.col.path");
    let key_width = [&name_label, &slug_label, &path_label]
        .iter()
        .map(|k| k.len())
        .max()
        .unwrap_or(0)
        + 1;

    println!("\n{}  {}", "⚑".yellow().bold(), t!("bottle.delete.header"));
    println!(
        "   {:<w$}│  {}",
        name_label.dimmed(),
        display_name,
        w = key_width
    );
    println!("   {:<w$}│  {}", slug_label.dimmed(), slug, w = key_width);
    println!(
        "   {:<w$}│  {}",
        path_label.dimmed(),
        db_path.dimmed(),
        w = key_width
    );
    println!();
}

/// Mensaje de error: bottle no encontrado al intentar borrarlo.
pub fn bottle_delete_not_found(slug: &str) {
    eprintln!(
        "\n{}  {}\n",
        "✗".red().bold(),
        t!("bottle.delete.not_found", slug = slug)
    );
}

/// Mensaje de error: el usuario introdujo un slug distinto al esperado.
pub fn bottle_delete_wrong_slug() {
    eprintln!(
        "\n{}  {}\n",
        "✗".red().bold(),
        t!("bottle.delete.wrong_slug")
    );
}

/// Confirma la eliminación del registro de un bottle.
pub fn bottle_delete_done(slug: &str) {
    let slug_label = t!("bottle.delete.col.slug");
    let key_width = slug_label.len() + 1;
    println!("\n{}  {}", "✓".green().bold(), t!("bottle.delete.done"));
    println!("   {:<w$}│  {}\n", slug_label.dimmed(), slug, w = key_width);
}

// ─── Bottle repair ────────────────────────────────────────────────────────────

/// Mensaje de error: bottle no encontrado al intentar repararlo.
pub fn bottle_repair_not_found(slug: &str) {
    eprintln!(
        "\n{}  {}\n",
        "✗".red().bold(),
        t!("bottle.repair.not_found", slug = slug)
    );
}

/// Mensaje de error: el bottle ya tiene una DB vinculada y existente.
pub fn bottle_repair_already_linked(slug: &str) {
    eprintln!(
        "\n{}  {}\n",
        "✗".red().bold(),
        t!("bottle.repair.already_linked", slug = slug)
    );
}

/// Cabecera del flujo de reparación: nombre del bottle y path actual roto.
pub fn bottle_repair_header(display_name: &str, current_path: &str) {
    println!("\n  {} {}", t!("bottle.repair.header"), display_name.bold());
    println!(
        "  {}    {}",
        t!("bottle.repair.current_path"),
        current_path.dimmed()
    );
    println!();
}

/// Mensaje de error: el nuevo path indicado no existe o no es un fichero.
pub fn bottle_repair_invalid_path() {
    eprintln!(
        "\n{}  {}\n",
        "✗".red().bold(),
        t!("bottle.repair.invalid_path")
    );
}

/// Confirma la reparación de un bottle con su nuevo path.
pub fn bottle_repair_done(slug: &str, new_path: &str) {
    let slug_label = t!("bottle.repair.col.slug");
    let path_label = t!("bottle.repair.col.path");
    let key_width = [&slug_label, &path_label]
        .iter()
        .map(|k| k.len())
        .max()
        .unwrap_or(0)
        + 1;

    println!("\n{}  {}", "✓".green().bold(), t!("bottle.repair.done"));
    println!("   {:<w$}│  {}", slug_label.dimmed(), slug, w = key_width);
    println!(
        "   {:<w$}│  {}\n",
        path_label.dimmed(),
        new_path,
        w = key_width
    );
}
