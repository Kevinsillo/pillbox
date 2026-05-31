//! Mensajes de instalación/desinstalación del componente MCP y skill.

use super::layout::print_b;
use owo_colors::OwoColorize;
use rust_i18n::t;

/// Confirma la desinstalación del componente MCP.
pub fn mcp_uninstalled() {
    print_b(&t!("mcp.uninstalled"));
}

/// Informa que el componente MCP no está instalado.
pub fn mcp_not_installed() {
    println!("\n{}\n", t!("mcp.not_installed").dimmed());
}

/// Confirma la desinstalación del skill.
pub fn skill_uninstalled() {
    print_b(&t!("skill.uninstalled"));
}

/// Informa que el skill no está instalado.
pub fn skill_not_installed() {
    println!("\n{}\n", t!("skill.not_installed").dimmed());
}
