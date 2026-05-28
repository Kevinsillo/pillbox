//! Subcomandos `pillbox skill *`.

use anyhow::Result;
use owo_colors::OwoColorize;
use rust_i18n::t;

use crate::output;

use super::install;

/// Descarga e instala la skill de Claude Code desde la última release de GitHub.
pub fn cmd_skill_install() -> Result<()> {
    let skill_path = pillbox::config::skill_path();
    let skill_dir = skill_path.parent().unwrap().to_path_buf();

    println!("\n  {}", t!("skill.downloading").dimmed());
    let version = install::skill::install(&skill_dir)?;

    output::fmt::skill_installed(&skill_path, &version);
    Ok(())
}

/// Muestra el estado de instalación de la skill junto a su ayuda.
pub fn cmd_skill_status() -> Result<()> {
    output::fmt::component_status_with_help(
        &pillbox::config::skill_path(),
        &crate::help::render_help("skill"),
    );
    Ok(())
}

/// Desinstala la skill eliminando su directorio.
pub fn cmd_skill_uninstall() -> Result<()> {
    let skill_dir = pillbox::config::skill_path()
        .parent()
        .unwrap()
        .to_path_buf();

    let removed = install::skill::uninstall(&skill_dir)?;
    if removed {
        output::fmt::skill_uninstalled();
    } else {
        output::fmt::skill_not_installed();
    }
    Ok(())
}
