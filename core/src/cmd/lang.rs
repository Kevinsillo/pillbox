use anyhow::Result;
use owo_colors::OwoColorize;
use rust_i18n::t;

use crate::{i18n, output};

pub fn cmd_lang_show(render_help: &str) -> Result<()> {
    let current = rust_i18n::locale().to_string();
    let rows: Vec<[String; 2]> = i18n::SUPPORTED
        .iter()
        .map(|(code, name)| {
            let code_col = if *code == current.as_str() {
                format!("{} {}", "●".green(), code)
            } else {
                format!("  {}", code)
            };
            [code_col, name.to_string()]
        })
        .collect();

    let current_name = i18n::SUPPORTED
        .iter()
        .find(|(c, _)| *c == current.as_str())
        .map(|(_, n)| *n)
        .unwrap_or(&current);

    println!(
        "\n{}: {} ({})\n",
        t!("lang.current").bold(),
        current_name,
        current
    );
    println!("{}\n", output::table::dict(rows));
    println!("{}", render_help);
    Ok(())
}

pub fn cmd_lang_set(code: String) -> Result<()> {
    let code = code.to_lowercase();
    if !i18n::is_supported(&code) {
        let available = i18n::SUPPORTED
            .iter()
            .map(|(c, _)| *c)
            .collect::<Vec<_>>()
            .join(", ");
        anyhow::bail!(
            "{}",
            t!("lang.set.invalid", lang = code, available = available)
        );
    }
    i18n::save(&code)?;
    rust_i18n::set_locale(&code);
    println!(
        "\n{} {}\n",
        "✓".green().bold(),
        t!("lang.set.ok", lang = code)
    );
    Ok(())
}
