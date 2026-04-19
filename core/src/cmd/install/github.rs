use anyhow::{Context, Result};
use serde::Deserialize;

use super::manifest::Manifest;

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
        .with_context(|| format!("no se pudo conectar con GitHub para {}", repo))?
        .error_for_status()
        .with_context(|| format!("GitHub devolvió error para {}", repo))?
        .json()
        .context("respuesta de GitHub no es JSON válido")?;
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
        .with_context(|| format!("no se pudo descargar {}", url))?
        .error_for_status()
        .with_context(|| format!("error al descargar {}", url))?
        .bytes()
        .context("error leyendo el cuerpo de la descarga")?;
    Ok(bytes.to_vec())
}

/// Descarga `pillbox.json` de la última release y lo deserializa.
pub fn fetch_manifest(repo: &str) -> Result<(String, Manifest)> {
    let version = latest_version(repo)?;
    let bytes = download_asset(repo, &version, "pillbox.json")?;
    let manifest: Manifest =
        serde_json::from_slice(&bytes).context("pillbox.json inválido o formato desconocido")?;
    Ok((version, manifest))
}
