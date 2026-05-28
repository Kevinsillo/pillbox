//! Logo ASCII de pillbox.

use owo_colors::OwoColorize;

const LOGO: [&str; 7] = [
    "████████              ████████",
    "███         ██████         ███",
    "███      ████████████      ███",
    "███      ▓▓▓▓▓▓▓▓▓▓▓▓      ███",
    "███      ▒▒▒▒▒▒▒▒▒▒▒▒      ███",
    "███         ▒▒▒▒▒▒         ███",
    "████████              ████████",
];

fn color_line(line: &str) -> String {
    let mut out = String::new();
    let mut seg = String::new();
    let mut red = false;
    for ch in line.chars() {
        let is_red = matches!(ch, '▓' | '▒');
        if is_red != red && !seg.is_empty() {
            out.push_str(&if red {
                seg.red().to_string()
            } else {
                seg.bright_white().to_string()
            });
            seg.clear();
        }
        red = is_red;
        seg.push(ch);
    }
    if !seg.is_empty() {
        out.push_str(&if red {
            seg.red().to_string()
        } else {
            seg.bright_white().to_string()
        });
    }
    out
}

/// Imprime el logo ASCII de pillbox junto con la versión del binario.
pub fn print_logo(version: &str) {
    for (i, line) in LOGO.iter().enumerate() {
        if i == 6 {
            println!(" {}  pillbox {}", color_line(line), version.dimmed());
        } else {
            println!(" {}", color_line(line));
        }
    }
    println!();
}
