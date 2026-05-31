//! Ejecución del manifest `pillbox.json` para instalar/desinstalar componentes.
//!
//! El manifest describe cómo instalar el asset descargado (tarball o fichero único)
//! y, opcionalmente, una entrada MCP neutral (comando + entry point) que el caller
//! registra en la config del proveedor elegido.
//!
//! Separación de responsabilidades:
//! - [`extract`] vuelca el asset en `dest_dir`. Es **genérico** (sin proveedor) y su
//!   contrato con el manifest remoto es byte-idéntico al histórico.
//! - [`register_mcp`] / [`unregister_mcp`] escriben/limpian la entrada MCP en la config
//!   de un [`Provider`] concreto, usando el esquema propio de cada uno.

use anyhow::{Context, Result};
use flate2::read::GzDecoder;
use pillbox::config::Provider;
use serde::Deserialize;
use std::path::Path;
use tar::Archive;

// ─── Tipos del manifest ───────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct Manifest {
    /// Nombre del asset a descargar del release (ej: "pillbox-mcp.tar.gz")
    pub asset: String,

    /// Cómo instalar el asset descargado
    #[serde(rename = "type")]
    pub install_type: InstallType,

    /// Acciones opcionales tras la instalación
    pub post_install: Option<PostInstall>,
}

#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InstallType {
    /// Archivo .tar.gz — se extrae en dest_dir
    Tarball,
    /// Archivo único — se copia en dest_dir con su nombre original
    File,
}

#[derive(Deserialize)]
pub struct PostInstall {
    /// Entrada MCP a registrar tras instalar.
    ///
    /// El nombre histórico `claude_json` se conserva por compatibilidad con el manifest
    /// remoto; su contenido es **neutral** (comando + entry point) y sirve para cualquier
    /// proveedor — el esquema concreto lo aplica [`register_mcp`].
    pub claude_json: Option<McpRegistration>,
}

#[derive(Deserialize)]
pub struct McpRegistration {
    /// Clave bajo la que vivía la entrada en el esquema histórico (ej: "pillbox").
    ///
    /// Hoy la clave efectiva la decide [`Provider::mcp_key`]; este campo se conserva
    /// por compatibilidad con el manifest remoto pero ya no se consulta.
    #[allow(dead_code)]
    pub key: String,
    /// Ejecutable (ej: "node")
    pub command: String,
    /// Ruta del entry point relativa a dest_dir (ej: "index.js")
    pub entry: String,
}

// ─── Extracción (genérica, sin proveedor) ──────────────────────────────────────

/// Vuelca los bytes descargados en `dest_dir` según el tipo de instalación del manifest.
///
/// No toca ninguna config de proveedor: el registro MCP es responsabilidad de
/// [`register_mcp`]. Contrato byte-idéntico al histórico (no recibe `Provider`).
///
/// - `bytes`    — contenido del asset descargado
/// - `dest_dir` — directorio de destino de la instalación
pub fn extract(manifest: &Manifest, bytes: &[u8], dest_dir: &Path) -> Result<()> {
    std::fs::create_dir_all(dest_dir)
        .with_context(|| format!("failed to create {}", dest_dir.display()))?;

    match manifest.install_type {
        InstallType::Tarball => {
            let gz = GzDecoder::new(bytes);
            let mut archive = Archive::new(gz);
            archive
                .unpack(dest_dir)
                .context("failed to extract tarball")?;
        }
        InstallType::File => {
            let filename = std::path::Path::new(&manifest.asset)
                .file_name()
                .context("asset has no valid filename")?;
            std::fs::write(dest_dir.join(filename), bytes).context("failed to write file")?;
        }
    }

    Ok(())
}

/// Devuelve la entrada MCP neutral declarada por el manifest, si existe.
pub fn mcp_entry(manifest: &Manifest) -> Option<&McpRegistration> {
    manifest
        .post_install
        .as_ref()
        .and_then(|p| p.claude_json.as_ref())
}

// ─── Registro MCP por proveedor ────────────────────────────────────────────────

/// Registra (o actualiza) la entrada MCP de Pillbox en la config del proveedor.
///
/// Esquema por proveedor:
/// - Claude Code: `~/.claude.json` → `mcpServers.pillbox = {command, args:[entry]}`
/// - OpenCode:    `~/.config/opencode/opencode.json` → `mcp.pillbox = {type:"local", command:[cmd, entry], enabled:true}`
///
/// Idempotente: una reinstalación deja exactamente una entrada, actualizada y sin
/// duplicar; el resto de claves del fichero se preservan.
pub fn register_mcp(
    provider: Provider,
    cfg_path: &Path,
    command: &str,
    entry_path: &Path,
) -> Result<()> {
    let entry = entry_value(provider, command, entry_path);
    set_config_key(cfg_path, provider.mcp_key(), entry)
}

/// Elimina la entrada MCP de Pillbox de la config del proveedor.
///
/// Solo borra la clave de Pillbox (`mcpServers.pillbox` o `mcp.pillbox`); el resto del
/// fichero queda intacto. No falla si el fichero o la clave no existen.
pub fn unregister_mcp(provider: Provider, cfg_path: &Path) -> Result<()> {
    remove_config_key(cfg_path, provider.mcp_key())
}

/// Construye el valor JSON de la entrada MCP según el esquema del proveedor.
fn entry_value(provider: Provider, command: &str, entry_path: &Path) -> serde_json::Value {
    let entry = entry_path.to_string_lossy();
    match provider {
        Provider::Claude => serde_json::json!({
            "command": command,
            "args": [entry],
        }),
        Provider::OpenCode => serde_json::json!({
            "type": "local",
            "command": [command, entry],
            "enabled": true,
        }),
    }
}

// ─── Helpers de escritura JSON con clave punteada ──────────────────────────────

/// Lee `cfg_path` como JSON (objeto vacío si no existe o es inválido), asigna `value`
/// bajo la `key` punteada (ej. `"mcpServers.pillbox"`) y reescribe el fichero.
fn set_config_key(cfg_path: &Path, key: &str, value: serde_json::Value) -> Result<()> {
    let mut root = read_json_root(cfg_path)?;

    let parts: Vec<&str> = key.splitn(2, '.').collect();
    match parts.as_slice() {
        [parent, child] => {
            root[parent][child] = value;
        }
        [single] => {
            root[single] = value;
        }
        _ => anyhow::bail!("invalid config key: {}", key),
    }

    write_json_root(cfg_path, &root)
}

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
    fn register_mcp_claude_writes_mcp_servers_schema() {
        let dir = tempfile::tempdir().unwrap();
        let cfg = dir.path().join(".claude.json");
        register_mcp(
            Provider::Claude,
            &cfg,
            "node",
            Path::new("/home/u/.pillbox/mcp/index.js"),
        )
        .unwrap();

        let root = read(&cfg);
        let entry = &root["mcpServers"]["pillbox"];
        assert_eq!(entry["command"], "node");
        assert_eq!(entry["args"][0], "/home/u/.pillbox/mcp/index.js");
        // El esquema de Claude NO usa los campos de OpenCode.
        assert!(entry.get("type").is_none());
        assert!(entry.get("enabled").is_none());
    }

    #[test]
    fn register_mcp_opencode_writes_local_command_array_schema() {
        let dir = tempfile::tempdir().unwrap();
        let cfg = dir.path().join("opencode.json");
        register_mcp(
            Provider::OpenCode,
            &cfg,
            "node",
            Path::new("/home/u/.pillbox/mcp/index.js"),
        )
        .unwrap();

        let root = read(&cfg);
        let entry = &root["mcp"]["pillbox"];
        assert_eq!(entry["type"], "local");
        assert_eq!(entry["enabled"], true);
        assert_eq!(entry["command"][0], "node");
        assert_eq!(entry["command"][1], "/home/u/.pillbox/mcp/index.js");
        // El esquema de OpenCode NO usa los campos de Claude.
        assert!(entry.get("args").is_none());
    }

    #[test]
    fn register_mcp_is_idempotent_single_entry() {
        let dir = tempfile::tempdir().unwrap();
        let cfg = dir.path().join(".claude.json");
        register_mcp(Provider::Claude, &cfg, "node", Path::new("/old/index.js")).unwrap();
        register_mcp(Provider::Claude, &cfg, "node", Path::new("/new/index.js")).unwrap();

        let root = read(&cfg);
        let servers = root["mcpServers"].as_object().unwrap();
        // Exactamente una entrada, actualizada al último valor.
        assert_eq!(servers.len(), 1);
        assert_eq!(root["mcpServers"]["pillbox"]["args"][0], "/new/index.js");
    }

    #[test]
    fn register_mcp_preserves_other_keys() {
        let dir = tempfile::tempdir().unwrap();
        let cfg = dir.path().join(".claude.json");
        std::fs::write(
            &cfg,
            r#"{"theme":"dark","mcpServers":{"other":{"command":"foo"}}}"#,
        )
        .unwrap();

        register_mcp(Provider::Claude, &cfg, "node", Path::new("/x/index.js")).unwrap();

        let root = read(&cfg);
        // Claves ajenas intactas.
        assert_eq!(root["theme"], "dark");
        assert_eq!(root["mcpServers"]["other"]["command"], "foo");
        // Pillbox añadida sin pisar lo demás.
        assert_eq!(root["mcpServers"]["pillbox"]["command"], "node");
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
        // Otras entradas MCP se conservan.
        assert!(mcp.get("keep").is_some());
    }

    #[test]
    fn unregister_mcp_no_file_is_noop() {
        let dir = tempfile::tempdir().unwrap();
        let cfg = dir.path().join("missing.json");
        // No debe fallar ni crear el fichero.
        unregister_mcp(Provider::Claude, &cfg).unwrap();
        assert!(!cfg.exists());
    }
}
