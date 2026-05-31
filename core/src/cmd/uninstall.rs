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

    if std::fs::remove_file(&bin_path).is_err() {
        eprintln!(
            "{} {}",
            "!".yellow().bold(),
            t!("uninstall.bin_error", path = bin_path.display())
        );
    }

    println!("{} {}\n", "✓".green().bold(), t!("uninstall.done"));
    Ok(())
}
