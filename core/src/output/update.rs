//! Mensajes de salida para el comando `pillbox update`.

use super::layout::{print_a, print_b, print_warn};
use super::table;
use owo_colors::OwoColorize;
use rust_i18n::t;

/// Informa de que el binario ya está en la última versión.
pub fn update_up_to_date(version: &str) {
    print_b(&t!("update.up_to_date", version = version));
}

/// Muestra la tabla con versión actual y nueva disponible.
pub fn update_available(current: &str, latest: &str) {
    let rows = vec![
        [
            t!("update.labels.current").bold().to_string(),
            current.to_string(),
        ],
        [
            t!("update.labels.latest").bold().to_string(),
            latest.green().to_string(),
        ],
    ];
    println!("\n{}", table::dict(rows));
}

/// Confirma que la actualización se completó correctamente.
pub fn update_done(version: &str, path: &str) {
    print_a(
        &t!("update.updated"),
        &[
            (t!("update.labels.version").as_ref(), version),
            (t!("update.labels.path").as_ref(), path),
        ],
    );
}

/// Avisa de que el servicio web no pudo reiniciarse tras la actualización.
#[cfg_attr(windows, allow(dead_code))]
pub fn update_restart_failed(err: &str) {
    print_warn(&t!("update.restart.failed", err = err));
}

/// Informa de que el servicio web se reiniciará cuando Pillbox termine (Windows).
#[cfg_attr(not(windows), allow(dead_code))]
pub fn update_restart_deferred() {
    println!("\n{}\n", t!("update.restart.deferred").dimmed());
}

/// Avisa de que el servicio instalado ejecuta un binario distinto del actualizado.
#[cfg_attr(windows, allow(dead_code))]
pub fn update_exe_mismatch(service: &str, current: &str) {
    print_warn(&t!(
        "update.warn.exe_mismatch",
        service = service,
        current = current
    ));
}
