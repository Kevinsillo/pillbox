//! Subcomando `pillbox update` — auto-actualización del binario.

use anyhow::{Context, Result};
use owo_colors::OwoColorize;
use rust_i18n::t;

use crate::output;
use super::install::github;

const REPO: &str = "Kevinsillo/pillbox";

/// Nombre del artifact de la release que corresponde a la plataforma actual.
fn self_artifact() -> Option<&'static str> {
    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    return Some("pillbox-linux-x86_64");
    #[cfg(all(target_os = "linux", target_arch = "aarch64"))]
    return Some("pillbox-linux-aarch64");
    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    return Some("pillbox-darwin-aarch64");
    #[cfg(all(target_os = "macos", target_arch = "x86_64"))]
    return Some("pillbox-darwin-x86_64");
    #[cfg(all(target_os = "windows", target_arch = "x86_64"))]
    return Some("pillbox-windows-x86_64.exe");
    #[cfg(all(target_os = "windows", target_arch = "aarch64"))]
    return Some("pillbox-windows-aarch64.exe");
    #[allow(unreachable_code)]
    None
}

pub fn run() -> Result<()> {
    let current = env!("CARGO_PKG_VERSION");

    println!("\n  {}", t!("update.checking").dimmed());

    let latest_tag = github::latest_version(REPO).context("failed to check latest version")?;
    let latest = latest_tag.trim_start_matches('v');

    if current == latest {
        output::fmt::update_up_to_date(&format!("v{}", current));
        return Ok(());
    }

    output::fmt::update_available(&format!("v{}", current), &format!("v{}", latest));

    let confirm = inquire::Confirm::new(&t!("update.confirm"))
        .with_default(true)
        .prompt()?;

    if !confirm {
        return Ok(());
    }

    let artifact = self_artifact().ok_or_else(|| {
        let platform = format!("{}/{}", std::env::consts::OS, std::env::consts::ARCH);
        anyhow::anyhow!(t!("update.error.no_binary", platform = platform))
    })?;

    println!("\n  {}", t!("update.downloading", version = &latest_tag).dimmed());

    let bytes = github::download_release_asset(REPO, &latest_tag, artifact)
        .context("failed to download update")?;

    let exe_path = std::env::current_exe().context("failed to locate current binary")?;
    replace_binary(&exe_path, &bytes)?;

    output::fmt::update_done(
        &format!("v{}", latest),
        &exe_path.display().to_string(),
    );

    Ok(())
}

/// Reemplaza el binario actual con los bytes descargados.
///
/// Unix: escribe en `<exe>.tmp`, da permisos de ejecución, y hace `rename()` atómico.
/// Windows: el exe en uso no se puede sobreescribir directamente; se escribe el nuevo
/// binario como `<exe>.new` y se lanza un proceso `cmd` diferido que lo mueve tras salir.
fn replace_binary(exe_path: &std::path::Path, bytes: &[u8]) -> Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let tmp = exe_path.with_extension("tmp");
        std::fs::write(&tmp, bytes)
            .with_context(|| format!("failed to write {}", tmp.display()))?;
        std::fs::set_permissions(&tmp, std::fs::Permissions::from_mode(0o755))
            .context("failed to set executable permissions")?;
        std::fs::rename(&tmp, exe_path)
            .with_context(|| t!("update.error.replace", err = "rename failed").to_string())?;
    }

    #[cfg(windows)]
    {
        let new_path = exe_path.with_extension("new");
        std::fs::write(&new_path, bytes)
            .with_context(|| format!("failed to write {}", new_path.display()))?;

        // Lanza un cmd diferido que espera a que este proceso termine y luego mueve el binario.
        std::process::Command::new("cmd")
            .args([
                "/C",
                "ping",
                "-n",
                "2",
                "127.0.0.1",
                ">nul",
                "&&",
                "move",
                "/Y",
                &new_path.to_string_lossy(),
                &exe_path.to_string_lossy(),
            ])
            .creation_flags(0x08000000) // CREATE_NO_WINDOW
            .spawn()
            .context("failed to launch deferred replace process")?;
    }

    Ok(())
}
