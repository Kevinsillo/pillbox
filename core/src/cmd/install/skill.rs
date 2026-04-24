use anyhow::{Context, Result};

use super::{github, manifest};

const REPO: &str = "kevinsillo/pillbox-skills";

/// Instala la skill de Claude Code desde la última release de GitHub usando el manifest.
pub fn install(dest_dir: &std::path::Path) -> Result<String> {
    let (version, mfst) = github::fetch_manifest(REPO).context("failed to fetch skill manifest")?;
    let bytes = github::download_asset(REPO, &version, &mfst.asset)
        .context("failed to download skill asset")?;
    manifest::install(&mfst, &bytes, dest_dir, None)?;
    Ok(version)
}

/// Desinstala la skill eliminando su directorio.
pub fn uninstall(dest_dir: &std::path::Path) -> Result<bool> {
    manifest::uninstall(dest_dir, None)
}
