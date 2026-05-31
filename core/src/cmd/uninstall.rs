//! Subcomando `pillbox uninstall`.

use anyhow::Result;
use owo_colors::OwoColorize;
use pillbox::config::Provider;
use rust_i18n::t;

use crate::cmd::install::manifest;
use crate::cmd::serve::{cmd_serve_uninstall, is_installed};
use crate::i18n::lang_file_path;

/// Desinstala Pillbox: muestra qué se va a eliminar, pide una sola confirmación y borra todo.
pub fn run() -> Result<()> {
    use inquire::Confirm;

    let bin_path = std::env::current_exe()?;

    // Construye la lista de componentes presentes
    let mcp_dir = pillbox::config::mcp_path().parent().unwrap().to_path_buf();
    // La skill puede estar instalada para cualquier proveedor (Claude, OpenCode, …).
    let skill_dirs: Vec<std::path::PathBuf> = Provider::ALL
        .iter()
        .map(|p| p.skill_path().parent().unwrap().to_path_buf())
        .collect();
    let global_db = pillbox::config::global_db_path();
    let lang_path = lang_file_path();
    let port_path = pillbox::config::serve_port_path();

    let has_serve = is_installed();
    let has_mcp = mcp_dir.exists();
    let has_skill = skill_dirs.iter().any(|d| d.exists());
    let has_db = global_db.exists();
    let has_config = lang_path.exists() || port_path.exists();

    // Muestra lo que se va a eliminar
    println!();
    if has_serve {
        println!("  {} {}", "·".dimmed(), t!("uninstall.items.serve"));
    }
    if has_mcp {
        println!("  {} {}", "·".dimmed(), t!("uninstall.items.mcp"));
    }
    if has_skill {
        println!("  {} {}", "·".dimmed(), t!("uninstall.items.skill"));
    }
    if has_db {
        println!("  {} {}", "·".dimmed(), t!("uninstall.items.db"));
    }
    if has_config {
        println!("  {} {}", "·".dimmed(), t!("uninstall.items.config"));
    }
    println!(
        "  {} {}",
        "·".dimmed(),
        t!("uninstall.items.bin", path = bin_path.display())
    );
    println!();

    if !Confirm::new(&t!("uninstall.prompt"))
        .with_default(false)
        .prompt()?
    {
        return Ok(());
    }

    println!();

    if has_serve {
        if let Err(e) = cmd_serve_uninstall() {
            eprintln!("{} {}", "!".yellow().bold(), e);
        }
    }

    if has_mcp {
        let _ = std::fs::remove_dir_all(&mcp_dir);
    }

    // Limpia la entrada MCP de Pillbox en la config de TODOS los proveedores.
    for provider in Provider::ALL {
        let _ = manifest::unregister_mcp(provider, &provider.mcp_config_path());
    }

    // Borra el directorio de skill de TODOS los proveedores.
    for skill_dir in &skill_dirs {
        if skill_dir.exists() {
            let _ = std::fs::remove_dir_all(skill_dir);
        }
    }

    if has_db {
        let _ = std::fs::remove_file(&global_db);
    }

    if has_config {
        let _ = std::fs::remove_file(&lang_path);
        let _ = std::fs::remove_file(&port_path);
    }

    // El binario: en Unix se desvincula en caliente (un binario en ejecución se
    // puede borrar); en Windows el `.exe` está bloqueado mientras corre, así que
    // se programa su borrado —y la limpieza del PATH de usuario que añadió
    // install.ps1— para que ocurra justo tras la salida del proceso.
    #[cfg(not(target_os = "windows"))]
    {
        if std::fs::remove_file(&bin_path).is_err() {
            eprintln!(
                "{} {}",
                "!".yellow().bold(),
                t!("uninstall.bin_error", path = bin_path.display())
            );
        }
    }
    #[cfg(target_os = "windows")]
    {
        match schedule_windows_cleanup(&bin_path) {
            Ok(()) => println!("  {} {}", "→".dimmed(), t!("uninstall.bin_scheduled")),
            Err(_) => eprintln!(
                "{} {}",
                "!".yellow().bold(),
                t!("uninstall.bin_error", path = bin_path.display())
            ),
        }
    }

    println!("{} {}\n", "✓".green().bold(), t!("uninstall.done"));
    Ok(())
}

/// Programa, solo en Windows, el borrado del binario y la limpieza de su entrada
/// en el PATH de usuario.
///
/// El `.exe` en ejecución no puede borrarse a sí mismo —Windows lo mantiene
/// bloqueado—, así que se escribe un helper PowerShell en `%TEMP%` y se lanza
/// **desacoplado**: espera a que el proceso libere el binario, lo elimina (y su
/// carpeta de instalación si queda vacía), quita ese directorio del PATH de
/// usuario (HKCU, deshaciendo lo que hizo install.ps1) y finalmente se
/// autoelimina.
#[cfg(target_os = "windows")]
fn schedule_windows_cleanup(bin_path: &std::path::Path) -> std::io::Result<()> {
    use std::os::windows::process::CommandExt;
    use std::process::Command;

    // DETACHED_PROCESS | CREATE_NO_WINDOW: el helper sobrevive a la salida de
    // pillbox y no abre ninguna ventana de consola.
    const DETACHED_PROCESS: u32 = 0x0000_0008;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;

    let install_dir = bin_path.parent().unwrap_or(bin_path);

    // PowerShell entrecomilla con comillas simples; se duplican las del path.
    let esc = |p: &std::path::Path| p.to_string_lossy().replace('\'', "''");
    let template = r#"$ErrorActionPreference = 'SilentlyContinue'
$exe = '__EXE__'
$dir = '__DIR__'
for ($i = 0; $i -lt 50 -and (Test-Path -LiteralPath $exe); $i++) {
    Start-Sleep -Milliseconds 200
    Remove-Item -LiteralPath $exe -Force
}
if ((Get-ChildItem -LiteralPath $dir -Force | Measure-Object).Count -eq 0) {
    Remove-Item -LiteralPath $dir -Recurse -Force
}
$userPath = [Environment]::GetEnvironmentVariable('Path', 'User')
if ($userPath) {
    $kept = $userPath -split ';' | Where-Object { $_ -ne '' -and $_.TrimEnd('\') -ne $dir.TrimEnd('\') }
    [Environment]::SetEnvironmentVariable('Path', ($kept -join ';'), 'User')
}
Remove-Item -LiteralPath $PSCommandPath -Force
"#;
    let script = template
        .replace("__EXE__", &esc(bin_path))
        .replace("__DIR__", &esc(install_dir));

    let helper = std::env::temp_dir().join("pillbox-uninstall.ps1");
    std::fs::write(&helper, script)?;

    Command::new("powershell")
        .args(["-NoProfile", "-ExecutionPolicy", "Bypass", "-File"])
        .arg(&helper)
        .creation_flags(DETACHED_PROCESS | CREATE_NO_WINDOW)
        .spawn()?;

    Ok(())
}
