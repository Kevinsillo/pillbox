//! Subcomandos `pillbox mcp *`.

use anyhow::Result;
use owo_colors::OwoColorize;
use rust_i18n::t;

use crate::output;

use super::install;

/// Descarga el repo MCP desde GitHub y ejecuta su script de instalación.
///
/// El script requiere Node.js >= 18, construye el bundle, lo instala y registra
/// la entrada MCP en los proveedores detectados.
pub fn cmd_mcp_install() -> Result<()> {
    let node_ok = std::process::Command::new("node")
        .arg("--version")
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .and_then(|v| {
            v.trim()
                .trim_start_matches('v')
                .split('.')
                .next()?
                .parse::<u32>()
                .ok()
        })
        .map(|maj| maj >= 18)
        .unwrap_or(false);

    if !node_ok {
        anyhow::bail!("{}", t!("mcp.node_required"));
    }

    println!("\n  {}", t!("mcp.downloading").dimmed());
    install::mcp::install()
}

/// Arranca el loop persistente NDJSON sobre stdin/stdout.
///
/// Bloquea hasta que el cliente cierre stdin (EOF). Toda la lógica vive
/// en `crate::mcp::run`; este wrapper sólo existe para enganchar el match
/// del subcomando en `main`.
pub fn cmd_mcp_run() -> Result<()> {
    crate::mcp::run()
}

/// Muestra el estado de instalación del servidor MCP junto a su ayuda.
pub fn cmd_mcp_status() -> Result<()> {
    output::fmt::component_status_with_help(
        &pillbox::config::mcp_path(),
        &crate::help::render_help("mcp"),
    );
    Ok(())
}

/// Desinstala el servidor MCP eliminando su directorio y la entrada del proveedor.
pub fn cmd_mcp_uninstall(provider: Option<String>) -> Result<()> {
    let provider = install::resolve_provider(provider.as_deref())?;
    let mcp_dir = pillbox::config::mcp_path().parent().unwrap().to_path_buf();

    let removed = install::mcp::uninstall(provider, &mcp_dir)?;
    if removed {
        output::fmt::mcp_uninstalled();
    } else {
        output::fmt::mcp_not_installed();
    }
    Ok(())
}
