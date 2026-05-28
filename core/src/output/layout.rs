//! Helpers transversales de layout (patrones A/B de confirmación).

use owo_colors::OwoColorize;

/// Pattern A — confirmation with inline key-value details.
pub(crate) fn print_a(action: &str, details: &[(&str, &str)]) {
    let key_width = details.iter().map(|(k, _)| k.len()).max().unwrap_or(0) + 1;
    println!("\n{} {}", "✓".green().bold(), action);
    for (key, val) in details {
        println!("   {:<width$}│  {}", key.dimmed(), val, width = key_width);
    }
    println!();
}

/// Pattern B — simple one-line confirmation.
pub(crate) fn print_b(action: &str) {
    println!("\n{} {}\n", "✓".green().bold(), action);
}
