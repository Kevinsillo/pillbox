use anyhow::{Context, Result};
use flate2::read::GzDecoder;
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
    /// Registrar una entrada en ~/.claude.json bajo mcpServers
    pub claude_json: Option<ClaudeJsonEntry>,
}

#[derive(Deserialize)]
pub struct ClaudeJsonEntry {
    /// Clave bajo mcpServers (ej: "pillbox")
    pub key: String,
    /// Ejecutable (ej: "node")
    pub command: String,
    /// Ruta del entry point relativa a dest_dir (ej: "index.js")
    pub entry: String,
}

// ─── Instalación ──────────────────────────────────────────────────────────────

/// Ejecuta el manifest sobre los bytes descargados.
///
/// - `bytes`      — contenido del asset descargado
/// - `dest_dir`   — directorio de destino de la instalación
/// - `claude_cfg` — ruta a ~/.claude.json (solo necesario si el manifest tiene post_install.claude_json)
pub fn install(
    manifest: &Manifest,
    bytes: &[u8],
    dest_dir: &Path,
    claude_cfg: Option<&Path>,
) -> Result<()> {
    std::fs::create_dir_all(dest_dir)
        .with_context(|| format!("no se pudo crear {}", dest_dir.display()))?;

    match manifest.install_type {
        InstallType::Tarball => {
            let gz = GzDecoder::new(bytes);
            let mut archive = Archive::new(gz);
            archive
                .unpack(dest_dir)
                .context("no se pudo extraer el tarball")?;
        }
        InstallType::File => {
            let filename = std::path::Path::new(&manifest.asset)
                .file_name()
                .context("el asset no tiene nombre de fichero válido")?;
            std::fs::write(dest_dir.join(filename), bytes)
                .context("no se pudo escribir el fichero")?;
        }
    }

    if let Some(post) = &manifest.post_install {
        if let Some(entry) = &post.claude_json {
            let cfg = claude_cfg
                .context("el manifest requiere claude_json pero no se proporcionó la ruta")?;
            write_claude_json(cfg, &entry.key, &entry.command, dest_dir.join(&entry.entry))?;
        }
    }

    Ok(())
}

fn write_claude_json(
    claude_cfg: &Path,
    key: &str,
    command: &str,
    entry_path: std::path::PathBuf,
) -> Result<()> {
    let mut root: serde_json::Value = if claude_cfg.exists() {
        let raw = std::fs::read_to_string(claude_cfg)
            .context("no se pudo leer ~/.claude.json")?;
        serde_json::from_str(&raw).unwrap_or(serde_json::json!({}))
    } else {
        serde_json::json!({})
    };

    // Navegar/crear la ruta de claves (ej: "mcpServers.pillbox")
    let parts: Vec<&str> = key.splitn(2, '.').collect();
    match parts.as_slice() {
        [parent, child] => {
            root[parent][child] = serde_json::json!({
                "command": command,
                "args": [entry_path.to_string_lossy()]
            });
        }
        [single] => {
            root[single] = serde_json::json!({
                "command": command,
                "args": [entry_path.to_string_lossy()]
            });
        }
        _ => anyhow::bail!("clave claude_json inválida: {}", key),
    }

    std::fs::write(claude_cfg, serde_json::to_string_pretty(&root)? + "\n")
        .context("no se pudo escribir ~/.claude.json")?;

    Ok(())
}

// ─── Desinstalación ───────────────────────────────────────────────────────────

/// Elimina dest_dir y opcionalmente limpia una clave de ~/.claude.json.
///
/// Devuelve `false` si dest_dir no existía (ya estaba desinstalado).
pub fn uninstall(
    dest_dir: &Path,
    claude_cfg_key: Option<(&str, &Path)>,
) -> Result<bool> {
    if !dest_dir.exists() {
        return Ok(false);
    }

    std::fs::remove_dir_all(dest_dir)
        .with_context(|| format!("no se pudo eliminar {}", dest_dir.display()))?;

    if let Some((key, claude_cfg)) = claude_cfg_key {
        if claude_cfg.exists() {
            let raw = std::fs::read_to_string(claude_cfg)
                .context("no se pudo leer ~/.claude.json")?;
            let mut root: serde_json::Value =
                serde_json::from_str(&raw).unwrap_or(serde_json::json!({}));
            let parts: Vec<&str> = key.splitn(2, '.').collect();
            if let [parent, child] = parts.as_slice() {
                if let Some(obj) = root[parent].as_object_mut() {
                    obj.remove(*child);
                }
            }
            std::fs::write(claude_cfg, serde_json::to_string_pretty(&root)? + "\n")
                .context("no se pudo actualizar ~/.claude.json")?;
        }
    }

    Ok(true)
}
