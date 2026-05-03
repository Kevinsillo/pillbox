//! Subcomando `pillbox uninstall`.

use anyhow::Result;
use owo_colors::OwoColorize;
use rust_i18n::t;

use crate::cmd::serve::{cmd_serve_uninstall, is_installed};

/// Desinstala interactivamente los componentes de Pillbox: servicio, MCP, skill, DB global y binario.
///
/// Cada componente pide confirmación por separado antes de eliminarlo.
pub fn run() -> Result<()> {
    use inquire::Confirm;

    // Servicio del sistema PRIMERO — si está instalado, debe desinstalarse antes
    // de borrar binarios o DB (best-effort: errores aquí no abortan el flujo).
    if is_installed()
        && Confirm::new(&t!("uninstall.serve.prompt"))
            .with_default(false)
            .prompt()?
    {
        match cmd_serve_uninstall() {
            Ok(_) => {
                // cmd_serve_uninstall ya imprime su propio mensaje de éxito.
            }
            Err(e) => {
                eprintln!("{} {}", "!".yellow().bold(), e);
            }
        }
    }

    let mcp_dir = pillbox::config::mcp_path().parent().unwrap().to_path_buf();
    if mcp_dir.exists() {
        if Confirm::new(&t!("uninstall.mcp.prompt"))
            .with_default(false)
            .prompt()?
        {
            std::fs::remove_dir_all(&mcp_dir)?;
            println!("\n{} {}\n", "✓".green().bold(), t!("uninstall.mcp.done"));
        }
    }

    let skill_dir = pillbox::config::skill_path()
        .parent()
        .unwrap()
        .to_path_buf();
    if skill_dir.exists() {
        if Confirm::new(&t!("uninstall.skill.prompt"))
            .with_default(false)
            .prompt()?
        {
            std::fs::remove_dir_all(&skill_dir)?;
            println!("\n{} {}\n", "✓".green().bold(), t!("uninstall.skill.done"));
        }
    }

    let global_db = pillbox::config::global_db_path();
    if global_db.exists() {
        if Confirm::new(&t!("uninstall.db.prompt"))
            .with_default(false)
            .prompt()?
        {
            std::fs::remove_file(&global_db)?;
            println!("\n{} {}\n", "✓".green().bold(), t!("uninstall.db.done"));
        }
    }

    let bin_path = std::env::current_exe()?;
    if Confirm::new(&t!("uninstall.bin.prompt", path = bin_path.display()))
        .with_default(false)
        .prompt()?
    {
        println!(
            "\n{}\n",
            t!("uninstall.bin.manual", path = bin_path.display())
        );
    }

    println!();
    Ok(())
}
