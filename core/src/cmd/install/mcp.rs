//! Instalación y desinstalación del servidor MCP desde GitHub.

use anyhow::{Context, Result};
use pillbox::config::Provider;

use super::{extract_and_run, github, manifest};

const REPO: &str = "kevinsillo/pillbox-mcp";

/// Descarga el repo MCP desde la rama `main` y ejecuta su script de instalación.
///
/// El script (`install.sh` / `install.ps1`) construye el bundle, lo copia a
/// `~/.pillbox/mcp/` y registra la entrada MCP en los proveedores detectados.
pub fn install() -> Result<()> {
    let bytes = github::download_repo_tarball(REPO)
        .context("failed to download MCP repository")?;
    extract_and_run(&bytes)
}

/// Desinstala el servidor MCP eliminando su directorio y la entrada del proveedor.
pub fn uninstall(provider: Provider, dest_dir: &std::path::Path) -> Result<bool> {
    manifest::unregister_mcp(provider, &provider.mcp_config_path())?;
    manifest::remove_dir(dest_dir)
}
