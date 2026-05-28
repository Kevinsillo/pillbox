//! Motor de renderizado de ayuda i18n para subcomandos `clap`.

use clap::CommandFactory;
use owo_colors::OwoColorize;
use rust_i18n::t;

use crate::cli::Cli;

/// Resuelve una clave i18n para la ayuda, usando `fallback` si la clave no está traducida.
fn t_help(key: &str, fallback: Option<String>) -> String {
    let translated = t!(key);
    if translated != key {
        translated.to_string()
    } else {
        fallback.unwrap_or_default()
    }
}

/// Renderiza la ayuda de un subcomando `clap` con traducciones i18n y colores ANSI.
/// - `display_name`: aparece en la línea "Uso: {display_name} [COMMAND]"
/// - `sub_prefix`: prefijo para el lookup de subcomandos en i18n (`help.sub.{sub_prefix}.{sub}`)
fn render_help_cmd(cmd: &mut clap::Command, display_name: &str, sub_prefix: &str) -> String {
    let mut out = String::new();
    let about_key = format!("help.about.{}", display_name);
    let about = t_help(&about_key, cmd.get_about().map(|a| a.to_string()));
    if !about.is_empty() {
        out.push_str(&format!("{}\n\n", about.bold()));
    }
    out.push_str(&format!(
        "{} {} {}\n",
        t!("help.usage").bold(),
        display_name,
        "[COMMAND]".dimmed()
    ));
    let subcmds: Vec<_> = cmd.get_subcommands().filter(|s| !s.is_hide_set()).collect();
    if !subcmds.is_empty() {
        out.push_str(&format!("\n{}:\n", t!("help.commands").bold()));
        let max = subcmds
            .iter()
            .map(|s| s.get_name().len())
            .max()
            .unwrap_or(0);
        for s in &subcmds {
            let sub_key = format!("help.sub.{}.{}", sub_prefix, s.get_name());
            let about = t_help(&sub_key, s.get_about().map(|a| a.to_string()));
            out.push_str(&format!(
                "  {:<width$}  {}\n",
                s.get_name().green(),
                about,
                width = max
            ));
        }
    }
    out
}

/// Renderiza la ayuda de un subcomando concreto de `pillbox`.
pub fn render_help(subcmd: &str) -> String {
    let mut cmd = Cli::command();
    let sub = cmd.find_subcommand_mut(subcmd).unwrap();
    render_help_cmd(sub, subcmd, subcmd)
}

/// Renderiza la ayuda de un subcomando anidado (ej. "bottle" → "migrate").
/// display_name usa la ruta completa; sub_prefix usa solo el nombre del hijo para el lookup i18n.
pub fn render_nested_help(parent: &str, child: &str) -> String {
    let mut cmd = Cli::command();
    let sub = cmd.find_subcommand_mut(parent).unwrap();
    let nested = sub.find_subcommand_mut(child).unwrap();
    render_help_cmd(nested, &format!("{} {}", parent, child), child)
}

/// Renderiza la ayuda raíz del binario `pillbox`.
pub fn render_root_help() -> String {
    let mut cmd = Cli::command();
    render_help_cmd(&mut cmd, "pillbox", "pillbox")
}
