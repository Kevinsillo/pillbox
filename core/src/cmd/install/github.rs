//! Acceso a la API de GitHub Releases para obtener versiones y descargar assets.

use anyhow::{Context, Result};
use serde::Deserialize;

use super::manifest::Manifest;

/// Respuesta mínima de la API de GitHub Releases.
#[derive(Deserialize)]
struct GithubRelease {
    pub tag_name: String,
}

/// Devuelve el tag de la última release de un repo de GitHub.
pub fn latest_version(repo: &str) -> Result<String> {
    let url = format!("https://api.github.com/repos/{}/releases/latest", repo);
    let release: GithubRelease = reqwest::blocking::Client::new()
        .get(&url)
        .header("User-Agent", "pillbox-cli")
        .send()
        .with_context(|| format!("failed to connect to GitHub for {}", repo))?
        .error_for_status()
        .with_context(|| format!("GitHub returned an error for {}", repo))?
        .json()
        .context("GitHub response is not valid JSON")?;
    Ok(release.tag_name)
}

/// Descarga un asset de una release de GitHub y devuelve los bytes.
pub fn download_asset(repo: &str, version: &str, asset: &str) -> Result<Vec<u8>> {
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

/// Descarga `pillbox.json` de la última release y lo deserializa.
pub fn fetch_manifest(repo: &str) -> Result<(String, Manifest)> {
    let version = latest_version(repo)?;
    let bytes = download_asset(repo, &version, "pillbox.json")?;
    let manifest: Manifest =
        serde_json::from_slice(&bytes).context("pillbox.json is invalid or has unknown format")?;
    Ok((version, manifest))
}
