//! Helpers para desinstalar componentes: eliminar directorios y limpiar configs de proveedor.

use anyhow::{Context, Result};
use pillbox::config::Provider;
use serde_json;
use std::path::Path;

// ─── Registro MCP por proveedor ────────────────────────────────────────────────

/// Elimina la entrada MCP de Pillbox de la config del proveedor.
///
/// Solo borra la clave de Pillbox (`mcpServers.pillbox` o `mcp.pillbox`); el resto del
/// fichero queda intacto. No falla si el fichero o la clave no existen.
pub fn unregister_mcp(provider: Provider, cfg_path: &Path) -> Result<()> {
    remove_config_key(cfg_path, provider.mcp_key())
}

// ─── Helpers de escritura JSON con clave punteada ──────────────────────────────

/// Elimina la `key` punteada del JSON de `cfg_path` y reescribe. No-op si no existe.
fn remove_config_key(cfg_path: &Path, key: &str) -> Result<()> {
    if !cfg_path.exists() {
        return Ok(());
    }
    let mut root = read_json_root(cfg_path)?;

    let parts: Vec<&str> = key.splitn(2, '.').collect();
    match parts.as_slice() {
        [parent, child] => {
            if let Some(obj) = root[parent].as_object_mut() {
                obj.remove(*child);
            }
        }
        [single] => {
            if let Some(obj) = root.as_object_mut() {
                obj.remove(*single);
            }
        }
        _ => anyhow::bail!("invalid config key: {}", key),
    }

    write_json_root(cfg_path, &root)
}

/// Lee la raíz JSON del fichero de config; objeto vacío si no existe o no parsea.
fn read_json_root(cfg_path: &Path) -> Result<serde_json::Value> {
    if cfg_path.exists() {
        let raw = std::fs::read_to_string(cfg_path)
            .with_context(|| format!("failed to read {}", cfg_path.display()))?;
        Ok(serde_json::from_str(&raw).unwrap_or_else(|_| serde_json::json!({})))
    } else {
        Ok(serde_json::json!({}))
    }
}

/// Reescribe la raíz JSON pretty-printed (con newline final), creando el directorio padre.
fn write_json_root(cfg_path: &Path, root: &serde_json::Value) -> Result<()> {
    if let Some(parent) = cfg_path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }
    std::fs::write(cfg_path, serde_json::to_string_pretty(root)? + "\n")
        .with_context(|| format!("failed to write {}", cfg_path.display()))?;
    Ok(())
}

// ─── Desinstalación de directorio ──────────────────────────────────────────────

/// Elimina `dest_dir` recursivamente. Devuelve `false` si no existía (ya desinstalado).
pub fn remove_dir(dest_dir: &Path) -> Result<bool> {
    if !dest_dir.exists() {
        return Ok(false);
    }
    std::fs::remove_dir_all(dest_dir)
        .with_context(|| format!("failed to remove {}", dest_dir.display()))?;
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn read(path: &Path) -> serde_json::Value {
        let raw = std::fs::read_to_string(path).unwrap();
        serde_json::from_str(&raw).unwrap()
    }

    #[test]
    fn unregister_mcp_removes_only_pillbox_key() {
        let dir = tempfile::tempdir().unwrap();
        let cfg = dir.path().join("opencode.json");
        std::fs::write(
            &cfg,
            r#"{"mcp":{"pillbox":{"type":"local"},"keep":{"type":"local"}}}"#,
        )
        .unwrap();

        unregister_mcp(Provider::OpenCode, &cfg).unwrap();

        let root = read(&cfg);
        let mcp = root["mcp"].as_object().unwrap();
        assert!(mcp.get("pillbox").is_none());
        assert!(mcp.get("keep").is_some());
    }

    #[test]
    fn unregister_mcp_no_file_is_noop() {
        let dir = tempfile::tempdir().unwrap();
        let cfg = dir.path().join("missing.json");
        unregister_mcp(Provider::Claude, &cfg).unwrap();
        assert!(!cfg.exists());
    }
}
