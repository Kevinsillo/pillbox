//! Pantallas del flujo de migración entre DB global y local.

use super::layout::print_a;
use super::table;
use owo_colors::OwoColorize;
use rust_i18n::t;

/// Muestra las instrucciones de ayuda para el comando `migrate` con las rutas de DB disponibles.
pub fn migrate_help(bottle_name: Option<&str>, local_path: &str, global_path: &str, help: &str) {
    let bottle_val = bottle_name
        .map(|n| n.to_string())
        .unwrap_or_else(|| t!("migrate.help.no_bottle").to_string());
    let rows = vec![
        [t!("migrate.help.bottle").bold().to_string(), bottle_val],
        [
            t!("migrate.help.local").bold().to_string(),
            local_path.to_string(),
        ],
        [
            t!("migrate.help.global").bold().to_string(),
            global_path.to_string(),
        ],
    ];
    println!("\n{}\n\n{}\n", table::dict(rows), help);
}

/// Muestra el resumen de confirmación antes de migrar datos de local a global.
pub fn migrate_confirm_global(
    bottle_name: &str,
    local_path: &str,
    global_path: &str,
    prescriptions: usize,
    pills: usize,
) {
    let rows = vec![
        [
            t!("migrate.global.bottle").bold().to_string(),
            bottle_name.to_string(),
        ],
        [
            t!("migrate.global.origin").bold().to_string(),
            local_path.to_string(),
        ],
        [
            t!("migrate.global.dest").bold().to_string(),
            global_path.to_string(),
        ],
        [
            t!("migrate.global.prescriptions").bold().to_string(),
            prescriptions.to_string(),
        ],
        [
            t!("migrate.global.pills").bold().to_string(),
            pills.to_string(),
        ],
    ];
    println!("{}\n", table::dict(rows));
    println!("  {}\n", t!("migrate.global.warning").dimmed());
}

/// Muestra el resumen de confirmación antes de migrar datos de global a local.
pub fn migrate_confirm_local(
    bottle_name: &str,
    local_path: &str,
    global_path: &str,
    prescriptions: usize,
    pills: usize,
    will_create: bool,
) {
    let dest_val = if will_create {
        format!("{}  {}", local_path, t!("migrate.local.dest_new").dimmed())
    } else {
        local_path.to_string()
    };
    let rows = vec![
        [
            t!("migrate.local.bottle").bold().to_string(),
            bottle_name.to_string(),
        ],
        [
            t!("migrate.local.origin").bold().to_string(),
            global_path.to_string(),
        ],
        [t!("migrate.local.dest").bold().to_string(), dest_val],
        [
            t!("migrate.local.prescriptions").bold().to_string(),
            prescriptions.to_string(),
        ],
        [
            t!("migrate.local.pills").bold().to_string(),
            pills.to_string(),
        ],
    ];
    println!("{}\n", table::dict(rows));
    println!("  {}\n", t!("migrate.local.warning").dimmed());
}

/// Confirma el resultado de una migración local→global con el recuento de registros movidos.
pub fn migrate_result_global(prescriptions: usize, pills: usize) {
    let p = prescriptions.to_string();
    let pi = pills.to_string();
    let done = t!("migrate.result.done").to_string();
    let deleted = t!("migrate.result.removed_local").to_string();
    print_a(
        &done,
        &[("prescriptions", &p), ("pills", &pi), ("deleted", &deleted)],
    );
}

/// Confirma el resultado de una migración global→local con el recuento de registros movidos.
pub fn migrate_result_local(prescriptions: usize, pills: usize) {
    let p = prescriptions.to_string();
    let pi = pills.to_string();
    let done = t!("migrate.result.done").to_string();
    let removed = t!("migrate.result.removed_global").to_string();
    print_a(
        &done,
        &[("prescriptions", &p), ("pills", &pi), ("removed", &removed)],
    );
}
