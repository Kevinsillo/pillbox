//! Lógica de instalación de componentes externos (MCP, skill) desde GitHub Releases.

pub mod github;
pub mod manifest;
pub mod mcp;
pub mod skill;

use anyhow::Result;
use pillbox::config::Provider;
use rust_i18n::t;
use std::io::IsTerminal;

/// Resuelve el [`Provider`] objetivo de una operación de instalación.
///
/// Estrategia (en orden):
/// 1. `--provider` explícito → se valida con [`Provider::parse`]; valor inválido es error.
/// 2. Detección: proveedores con su config presente ([`Provider::detect`]).
///    - 0 detectados → error "ninguno detectado".
/// 3. Sin flag y ≥1 detectado:
///    - non-TTY → error pidiendo `--provider` (NUNCA bloquea con un prompt).
///    - TTY → prompt `inquire::Select` **siempre** (aunque solo haya uno).
pub fn resolve_provider(flag: Option<&str>) -> Result<Provider> {
    // 1. Flag explícito.
    if let Some(value) = flag {
        return Provider::parse(value)
            .ok_or_else(|| anyhow::anyhow!(t!("provider.invalid", value = value)));
    }

    // 2. Detección.
    let detected = Provider::detect();
    if detected.is_empty() {
        anyhow::bail!("{}", t!("provider.none_detected"));
    }

    // 3a. Sin TTY no podemos preguntar: exigir --provider en vez de colgarnos.
    if !std::io::stdin().is_terminal() {
        anyhow::bail!("{}", t!("provider.non_tty"));
    }

    // 3b. Prompt SIEMPRE (incluso con un único detectado) para confirmar el destino.
    let labels: Vec<&str> = detected.iter().map(|p| p.label()).collect();
    let choice = inquire::Select::new(&t!("provider.select"), labels).prompt()?;
    let provider = detected
        .iter()
        .find(|p| p.label() == choice)
        .copied()
        .expect("selection must match a detected provider");
    Ok(provider)
}
