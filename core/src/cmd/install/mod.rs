//! Lógica de instalación de componentes externos (MCP, skill) desde GitHub.

pub mod github;
pub mod manifest;
pub mod mcp;
pub mod skill;

use anyhow::{Context, Result};
use flate2::read::GzDecoder;
use pillbox::config::Provider;
use rust_i18n::t;
use std::io::IsTerminal;
use std::path::Path;
use tar::Archive;

/// Resuelve el [`Provider`] objetivo de una operación de instalación/desinstalación.
///
/// Estrategia (en orden):
/// 1. `--provider` explícito → se valida con [`Provider::parse`]; valor inválido es error.
/// 2. Detección: proveedores con su config presente ([`Provider::detect`]).
///    - 0 detectados → error "ninguno detectado".
/// 3. Sin flag y ≥1 detectado:
///    - non-TTY → error pidiendo `--provider` (NUNCA bloquea con un prompt).
///    - TTY → prompt `inquire::Select` **siempre** (aunque solo haya uno).
pub fn resolve_provider(flag: Option<&str>) -> Result<Provider> {
    // 1. Flag explícito.
    if let Some(value) = flag {
        return Provider::parse(value)
            .ok_or_else(|| anyhow::anyhow!(t!("provider.invalid", value = value)));
    }

    // 2. Detección.
    let detected = Provider::detect();
    if detected.is_empty() {
        anyhow::bail!("{}", t!("provider.none_detected"));
    }

    // 3a. Sin TTY no podemos preguntar: exigir --provider en vez de colgarnos.
    if !std::io::stdin().is_terminal() {
        anyhow::bail!("{}", t!("provider.non_tty"));
    }

    // 3b. Prompt SIEMPRE (incluso con un único detectado) para confirmar el destino.
    let labels: Vec<&str> = detected.iter().map(|p| p.label()).collect();
    let choice = inquire::Select::new(&t!("provider.select"), labels).prompt()?;
    let provider = detected
        .iter()
        .find(|p| p.label() == choice)
        .copied()
        .expect("selection must match a detected provider");
    Ok(provider)
}

/// Extrae el tarball de un repo GitHub en un directorio temporal y ejecuta el
/// script de instalación del SO (`install.sh` en Unix, `install.ps1` en Windows).
///
/// El tarball de GitHub siempre contiene un único directorio raíz con prefijo
/// `Owner-repo-sha/`; esta función lo localiza y pasa su ruta al script.
/// El directorio temporal se limpia automáticamente al salir.
pub(super) fn extract_and_run(bytes: &[u8]) -> Result<()> {
    let tmp = tempfile::tempdir().context("failed to create temp directory")?;

    let gz = GzDecoder::new(bytes);
    let mut archive = Archive::new(gz);
    archive
        .unpack(tmp.path())
        .context("failed to extract repo tarball")?;

    let root = std::fs::read_dir(tmp.path())
        .context("failed to read temp directory")?
        .filter_map(|e| e.ok())
        .find(|e| e.file_type().map(|t| t.is_dir()).unwrap_or(false))
        .context("no root directory found in tarball")?
        .path();

    run_install_script(&root)
}

/// Ejecuta el script de instalación desde `dir` con stdin/stdout/stderr heredados
/// para que el usuario pueda interactuar con él.
fn run_install_script(dir: &Path) -> Result<()> {
    #[cfg(unix)]
    let status = std::process::Command::new("sh")
        .arg("install.sh")
        .current_dir(dir)
        .status()
        .context("failed to run install.sh")?;

    #[cfg(windows)]
    let status = std::process::Command::new("powershell")
        .args(["-ExecutionPolicy", "Bypass", "-File", "install.ps1"])
        .current_dir(dir)
        .status()
        .context("failed to run install.ps1")?;

    if !status.success() {
        anyhow::bail!("install script exited with non-zero status");
    }
    Ok(())
}
