//! Descarga de tarballs y assets desde repositorios GitHub.

use anyhow::{Context, Result};
use serde::Deserialize;

// ─── Versión de la última release ─────────────────────────────────────────────

#[derive(Deserialize)]
struct GithubRelease {
    pub tag_name: String,
}

/// Devuelve el tag de la última release de un repo de GitHub (ej. `"v0.14.3"`).
pub fn latest_version(repo: &str) -> Result<String> {
    let url = format!("https://api.github.com/repos/{}/releases/latest", repo);
    let release: GithubRelease = reqwest::blocking::Client::new()
        .get(&url)
        .header("User-Agent", "pillbox-cli")
        .header("Accept", "application/vnd.github+json")
        .send()
        .with_context(|| format!("failed to connect to GitHub for {}", repo))?
        .error_for_status()
        .with_context(|| format!("GitHub returned an error for {}", repo))?
        .json()
        .context("GitHub response is not valid JSON")?;
    Ok(release.tag_name)
}

// ─── Descarga de assets de release ────────────────────────────────────────────

/// Descarga un asset de una release de GitHub y devuelve los bytes.
pub fn download_release_asset(repo: &str, version: &str, asset: &str) -> Result<Vec<u8>> {
    let url = format!(
        "https://github.com/{}/releases/download/{}/{}",
        repo, version, asset
    );
    let bytes = reqwest::blocking::Client::new()
        .get(&url)
        .header("User-Agent", "pillbox-cli")
        .send()
        .with_context(|| format!("failed to download {}", url))?
        .error_for_status()
        .with_context(|| format!("download failed for {}", url))?
        .bytes()
        .context("failed to read download body")?;
    Ok(bytes.to_vec())
}

// ─── Descarga de tarball de rama main ─────────────────────────────────────────

/// Descarga el tarball de la rama `main` de un repo de GitHub y devuelve los bytes.
///
/// El endpoint `/tarball/main` sigue la redirección automáticamente y devuelve
/// un `.tar.gz` con todos los ficheros del repo bajo un directorio raíz con prefijo
/// `Owner-repo-sha/`.
pub fn download_repo_tarball(repo: &str) -> Result<Vec<u8>> {
    let url = format!("https://api.github.com/repos/{}/tarball/main", repo);
    let bytes = reqwest::blocking::Client::new()
        .get(&url)
        .header("User-Agent", "pillbox-cli")
        .header("Accept", "application/vnd.github+json")
        .send()
        .with_context(|| format!("failed to connect to GitHub for {}", repo))?
        .error_for_status()
        .with_context(|| format!("GitHub returned an error for {}", repo))?
        .bytes()
        .context("failed to read download body")?;
    Ok(bytes.to_vec())
}
