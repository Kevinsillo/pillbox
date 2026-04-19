use anyhow::{Context, Result};

use super::{github, manifest};

const REPO: &str = "kevinsillo/pillbox-mcp";

/// Instala el servidor MCP desde la última release de GitHub usando el manifest.
pub fn install(dest_dir: &std::path::Path, claude_cfg: &std::path::Path) -> Result<String> {
    let (version, mfst) =
        github::fetch_manifest(REPO).context("no se pudo obtener el manifest del MCP")?;
    let bytes = github::download_asset(REPO, &version, &mfst.asset)
        .context("no se pudo descargar el MCP")?;
    manifest::install(&mfst, &bytes, dest_dir, Some(claude_cfg))?;
    Ok(version)
}

/// Desinstala el servidor MCP eliminando su directorio y la entrada en `~/.claude.json`.
pub fn uninstall(dest_dir: &std::path::Path, claude_cfg: &std::path::Path) -> Result<bool> {
    manifest::uninstall(dest_dir, Some(("mcpServers.pillbox", claude_cfg)))
}
