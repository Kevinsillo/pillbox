//! Subcomandos `pillbox skill *`.

use anyhow::Result;
use owo_colors::OwoColorize;
use rust_i18n::t;

use crate::output;

use super::install;

/// Descarga el repo de skills desde GitHub y ejecuta su script de instalación.
///
/// El script detecta los proveedores instalados e instala de forma interactiva.
pub fn cmd_skill_install() -> Result<()> {
    println!("\n  {}", t!("skill.downloading").dimmed());
    install::skill::install()
}

/// Muestra el estado de instalación de la skill junto a su ayuda.
pub fn cmd_skill_status() -> Result<()> {
    output::fmt::component_status_with_help(
        &pillbox::config::skill_path(),
        &crate::help::render_help("skill"),
    );
    Ok(())
}

/// Desinstala la skill eliminando su directorio para el proveedor indicado.
pub fn cmd_skill_uninstall(provider: Option<String>) -> Result<()> {
    let provider = install::resolve_provider(provider.as_deref())?;
    let skill_dir = provider.skill_path().parent().unwrap().to_path_buf();

    let removed = install::skill::uninstall(&skill_dir)?;
    if removed {
        output::fmt::skill_uninstalled();
    } else {
        output::fmt::skill_not_installed();
    }
    Ok(())
}
