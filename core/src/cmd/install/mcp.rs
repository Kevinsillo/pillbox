use anyhow::{Context, Result};
use flate2::read::GzDecoder;
use tar::Archive;

use super::github;

const REPO: &str = "kevinsillo/pillbox-mcp";
const ASSET: &str = "pillbox-mcp.tar.gz";

/// Instala el servidor MCP desde la última release de GitHub.
///
/// Descarga `pillbox-mcp.tar.gz`, lo extrae en `dest_dir` y registra
/// la entrada en `~/.claude.json`.
pub fn install(dest_dir: &std::path::Path, claude_cfg: &std::path::Path) -> Result<String> {
    let version = github::latest_version(REPO)
        .context("no se pudo obtener la versión más reciente del MCP")?;

    let bytes = github::download_asset(REPO, &version, ASSET)
        .context("no se pudo descargar el MCP")?;

    std::fs::create_dir_all(dest_dir)
        .with_context(|| format!("no se pudo crear {}", dest_dir.display()))?;

    let gz = GzDecoder::new(bytes.as_slice());
    let mut archive = Archive::new(gz);
    archive
        .unpack(dest_dir)
        .context("no se pudo extraer el tarball del MCP")?;

    register_claude_json(dest_dir, claude_cfg)?;

    Ok(version)
}

fn register_claude_json(
    mcp_dir: &std::path::Path,
    claude_cfg: &std::path::Path,
) -> Result<()> {
    let index_js = mcp_dir.join("index.js");
    let entry = serde_json::json!({
        "command": "node",
        "args": [index_js.to_string_lossy()]
    });

    let mut root: serde_json::Value = if claude_cfg.exists() {
        let raw = std::fs::read_to_string(claude_cfg)
            .context("no se pudo leer ~/.claude.json")?;
        serde_json::from_str(&raw).unwrap_or(serde_json::json!({}))
    } else {
        serde_json::json!({})
    };

    root["mcpServers"]["pillbox"] = entry;
    std::fs::write(claude_cfg, serde_json::to_string_pretty(&root)? + "\n")
        .context("no se pudo escribir ~/.claude.json")?;

    Ok(())
}

/// Desinstala el servidor MCP eliminando su directorio y la entrada en `~/.claude.json`.
pub fn uninstall(dest_dir: &std::path::Path, claude_cfg: &std::path::Path) -> Result<bool> {
    if !dest_dir.exists() {
        return Ok(false);
    }
    std::fs::remove_dir_all(dest_dir)
        .with_context(|| format!("no se pudo eliminar {}", dest_dir.display()))?;

    if claude_cfg.exists() {
        let raw = std::fs::read_to_string(claude_cfg)
            .context("no se pudo leer ~/.claude.json")?;
        let mut root: serde_json::Value =
            serde_json::from_str(&raw).unwrap_or(serde_json::json!({}));
        if let Some(servers) = root["mcpServers"].as_object_mut() {
            servers.remove("pillbox");
        }
        std::fs::write(claude_cfg, serde_json::to_string_pretty(&root)? + "\n")
            .context("no se pudo actualizar ~/.claude.json")?;
    }

    Ok(true)
}
