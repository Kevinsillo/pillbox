//! Mensajes de instalación/desinstalación del componente MCP y skill.

use super::layout::{print_a, print_b};
use owo_colors::OwoColorize;
use rust_i18n::t;

/// Confirma la instalación del componente MCP mostrando ruta, config y versión.
pub fn mcp_installed(path: &std::path::Path, config: &std::path::Path, version: &str) {
    let path_val = path.display().to_string().cyan().to_string();
    let cfg_val = config.display().to_string().cyan().to_string();
    print_a(
        &t!("mcp.installed"),
        &[
            ("path", &path_val),
            ("config", &cfg_val),
            ("version", version),
        ],
    );
}

/// Confirma la desinstalación del componente MCP.
pub fn mcp_uninstalled() {
    print_b(&t!("mcp.uninstalled"));
}

/// Informa que el componente MCP no está instalado.
pub fn mcp_not_installed() {
    println!("\n{}\n", t!("mcp.not_installed").dimmed());
}

/// Confirma la instalación del skill mostrando ruta y versión.
pub fn skill_installed(path: &std::path::Path, version: &str) {
    let path_val = path.display().to_string().cyan().to_string();
    print_a(
        &t!("skill.installed"),
        &[("path", &path_val), ("version", version)],
    );
}

/// Confirma la desinstalación del skill.
pub fn skill_uninstalled() {
    print_b(&t!("skill.uninstalled"));
}

/// Informa que el skill no está instalado.
pub fn skill_not_installed() {
    println!("\n{}\n", t!("skill.not_installed").dimmed());
}
