//! Subcomando `pillbox update` — auto-actualización del binario.

use anyhow::{Context, Result};
use owo_colors::OwoColorize;
use rust_i18n::t;

use super::install::github;
use super::serve::{self, RestartPlan};
use crate::output;

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

    println!(
        "\n  {}",
        t!("update.downloading", version = &latest_tag).dimmed()
    );

    let bytes = github::download_release_asset(REPO, &latest_tag, artifact)
        .context("failed to download update")?;

    let exe_path = std::env::current_exe().context("failed to locate current binary")?;

    // La decisión de reiniciar el servicio web se toma antes de reemplazar.
    let plan = serve::restart_plan(serve::is_installed(), serve::is_running());

    #[cfg(unix)]
    if let Some(service_exe) = serve::installed_service_exe() {
        if serve::exe_matches(Some(&service_exe), &exe_path) == Some(false) {
            output::fmt::update_exe_mismatch(
                &service_exe.display().to_string(),
                &exe_path.display().to_string(),
            );
        }
    }

    replace_binary(&exe_path, &bytes, plan == RestartPlan::Restart)?;

    output::fmt::update_done(&format!("v{}", latest), &exe_path.display().to_string());

    if plan == RestartPlan::Restart {
        restart_web_service();
    }

    Ok(())
}

/// Reinicia el servicio web tras reemplazar el binario.
///
/// Unix: reinicio inmediato; un fallo solo produce un aviso (la
/// actualización ya se completó). Windows: el script diferido de
/// `replace_binary` se encarga; aquí solo se informa.
fn restart_web_service() {
    #[cfg(unix)]
    match serve::restart_service() {
        Ok(()) => output::fmt::serve_restarted(&serve::serve_url()),
        Err(e) => output::fmt::update_restart_failed(&format!("{:#}", e)),
    }

    #[cfg(windows)]
    output::fmt::update_restart_deferred();
}

/// Reemplaza el binario actual con los bytes descargados.
///
/// Unix: escribe en `<exe>.tmp`, da permisos de ejecución, y hace `rename()` atómico.
/// Windows: el exe en uso no se puede sobreescribir directamente; se escribe el nuevo
/// binario como `<exe>.new` y se lanza un proceso `cmd` diferido que lo mueve tras salir.
/// Si `restart` es `true`, el script diferido además detiene el servicio web antes
/// del `move` y lo vuelve a arrancar después (ver [`windows_replace_script`]).
fn replace_binary(exe_path: &std::path::Path, bytes: &[u8], restart: bool) -> Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        // En Unix el reinicio se hace inline tras el reemplazo.
        let _ = restart;
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
        use std::os::windows::process::CommandExt;

        let new_path = exe_path.with_extension("new");
        std::fs::write(&new_path, bytes)
            .with_context(|| format!("failed to write {}", new_path.display()))?;

        // Lanza un cmd diferido que espera a que este proceso termine y luego mueve el binario.
        // `raw_arg` evita que el script (con sus propias comillas) se vuelva a entrecomillar.
        let script = windows_replace_script(exe_path, &new_path, restart);
        std::process::Command::new("cmd")
            .arg("/C")
            .raw_arg(script)
            .creation_flags(0x08000000) // CREATE_NO_WINDOW
            .spawn()
            .context("failed to launch deferred replace process")?;
    }

    Ok(())
}

/// Script de `cmd /C` que reemplaza el binario en Windows una vez termina
/// este proceso (`ping` como espera).
///
/// - `restart == false`: espera → `move /Y <new> <exe>`.
/// - `restart == true`: espera → `<exe> serve stop` → espera → `move` →
///   `<exe> serve start`. Los pasos tras el stop se encadenan con `&` para que
///   el arranque se intente aunque el `move` falle.
#[cfg_attr(not(windows), allow(dead_code))]
fn windows_replace_script(
    exe_path: &std::path::Path,
    new_path: &std::path::Path,
    restart: bool,
) -> String {
    let exe = exe_path.display();
    let new = new_path.display();
    let wait = "ping -n 2 127.0.0.1 >nul";
    if restart {
        format!(
            "{wait} && \"{exe}\" serve stop & {wait} & move /Y \"{new}\" \"{exe}\" & \"{exe}\" serve start"
        )
    } else {
        format!("{wait} && move /Y \"{new}\" \"{exe}\"")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn windows_replace_script_without_restart_only_moves() {
        let script = windows_replace_script(
            Path::new(r"C:\Program Files\pillbox\pillbox.exe"),
            Path::new(r"C:\Program Files\pillbox\pillbox.new"),
            false,
        );
        assert_eq!(
            script,
            r#"ping -n 2 127.0.0.1 >nul && move /Y "C:\Program Files\pillbox\pillbox.new" "C:\Program Files\pillbox\pillbox.exe""#
        );
        assert!(!script.contains("serve"));
    }

    #[test]
    fn windows_replace_script_with_restart_stops_moves_and_starts() {
        let script = windows_replace_script(
            Path::new(r"C:\pb\pillbox.exe"),
            Path::new(r"C:\pb\pillbox.new"),
            true,
        );
        assert_eq!(
            script,
            r#"ping -n 2 127.0.0.1 >nul && "C:\pb\pillbox.exe" serve stop & ping -n 2 127.0.0.1 >nul & move /Y "C:\pb\pillbox.new" "C:\pb\pillbox.exe" & "C:\pb\pillbox.exe" serve start"#
        );
        let stop = script.find("serve stop").unwrap();
        let mv = script.find("move /Y").unwrap();
        let start = script.find("serve start").unwrap();
        assert!(stop < mv && mv < start);
    }
}
