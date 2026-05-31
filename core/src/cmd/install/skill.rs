//! Instalación y desinstalación de skills desde GitHub.

use anyhow::{Context, Result};

use super::{extract_and_run, github, manifest};

const REPO: &str = "kevinsillo/pillbox-skills";

/// Descarga el repo de skills desde la rama `main` y ejecuta su script de instalación.
///
/// El script (`install.sh` / `install.ps1`) detecta los proveedores instalados,
/// pregunta al usuario qué instalar y copia skills, agents y commands a sus rutas.
pub fn install() -> Result<()> {
    let bytes = github::download_repo_tarball(REPO)
        .context("failed to download skills repository")?;
    extract_and_run(&bytes)
}

/// Desinstala la skill eliminando su directorio en el proveedor indicado.
pub fn uninstall(dest_dir: &std::path::Path) -> Result<bool> {
    manifest::remove_dir(dest_dir)
}
