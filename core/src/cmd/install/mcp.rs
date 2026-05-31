//! Instalación y desinstalación del servidor MCP desde GitHub Releases.

use anyhow::{Context, Result};
use pillbox::config::Provider;

use super::{github, manifest};

const REPO: &str = "kevinsillo/pillbox-mcp";

/// Instala el servidor MCP desde la última release de GitHub y registra su entrada
/// en la config del proveedor indicado.
pub fn install(provider: Provider, dest_dir: &std::path::Path) -> Result<String> {
    let (version, mfst) = github::fetch_manifest(REPO).context("failed to fetch MCP manifest")?;
    let bytes = github::download_asset(REPO, &version, &mfst.asset)
        .context("failed to download MCP asset")?;

    manifest::extract(&mfst, &bytes, dest_dir)?;

    if let Some(entry) = manifest::mcp_entry(&mfst) {
        manifest::register_mcp(
            provider,
            &provider.mcp_config_path(),
            &entry.command,
            &dest_dir.join(&entry.entry),
        )?;
    }

    Ok(version)
}

/// Desinstala el servidor MCP eliminando su directorio y la entrada del proveedor.
pub fn uninstall(provider: Provider, dest_dir: &std::path::Path) -> Result<bool> {
    manifest::unregister_mcp(provider, &provider.mcp_config_path())?;
    manifest::remove_dir(dest_dir)
}
