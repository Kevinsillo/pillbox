use anyhow::{Context, Result};

use super::github;

const REPO: &str = "kevinsillo/pillbox-skills";
const ASSET: &str = "SKILL.md";

/// Instala la skill de Claude Code desde la última release de GitHub.
///
/// Descarga `SKILL.md` y lo coloca en `dest_dir/SKILL.md`.
pub fn install(dest_dir: &std::path::Path) -> Result<String> {
    let version = github::latest_version(REPO)
        .context("no se pudo obtener la versión más reciente de la skill")?;

    let bytes = github::download_asset(REPO, &version, ASSET)
        .context("no se pudo descargar la skill")?;

    std::fs::create_dir_all(dest_dir)
        .with_context(|| format!("no se pudo crear {}", dest_dir.display()))?;

    std::fs::write(dest_dir.join(ASSET), &bytes)
        .context("no se pudo escribir SKILL.md")?;

    Ok(version)
}

/// Desinstala la skill eliminando su directorio.
pub fn uninstall(dest_dir: &std::path::Path) -> Result<bool> {
    if !dest_dir.exists() {
        return Ok(false);
    }
    std::fs::remove_dir_all(dest_dir)
        .with_context(|| format!("no se pudo eliminar {}", dest_dir.display()))?;
    Ok(true)
}
