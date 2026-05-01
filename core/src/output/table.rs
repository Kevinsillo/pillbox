//! Constructores de tablas para la salida en terminal usando `tabled`.

use owo_colors::OwoColorize;
use tabled::{builder::Builder, settings::Style};

/// Pattern C — borderless list table with dim header and separator line.
pub fn plain_list(headers: &[&str], rows: Vec<Vec<String>>) -> String {
    let mut b = Builder::default();
    b.push_record(headers.iter().map(|h| h.dimmed().to_string()));
    for row in rows {
        b.push_record(row);
    }

    let raw = b.build().with(Style::blank()).to_string();

    let mut result = String::new();
    let mut first = true;
    for line in raw.lines() {
        result.push_str("  ");
        result.push_str(line);
        result.push('\n');
        if first {
            let width = ansi_stripped_len(line.trim_end());
            result.push_str("  ");
            result.push_str(&"─".repeat(width));
            result.push('\n');
            first = false;
        }
    }
    result
}

/// Pattern D — key/value dict table with modern_rounded style.
pub fn dict(rows: Vec<[String; 2]>) -> String {
    let mut b = Builder::default();
    for row in rows {
        b.push_record(row);
    }
    b.build().with(Style::modern_rounded()).to_string()
}

/// Calcula la longitud visible de una cadena ignorando las secuencias de escape ANSI.
fn ansi_stripped_len(s: &str) -> usize {
    let mut len = 0usize;
    let mut chars = s.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '\x1b' {
            for c in chars.by_ref() {
                if c == 'm' {
                    break;
                }
            }
        } else {
            len += 1;
        }
    }
    len
}
