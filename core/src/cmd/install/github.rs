//! Descarga de tarballs de repositorios GitHub (rama main, sin releases).

use anyhow::{Context, Result};

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
