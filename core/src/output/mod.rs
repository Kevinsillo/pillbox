//! Utilidades de presentación en terminal: formato, tablas y truncado de texto.

pub mod fmt;
pub(crate) mod table;

/// Trunca una cadena Unicode a un máximo de caracteres, añadiendo `…` si se recorta.
pub(crate) fn truncate(s: &str, max: usize) -> String {
    let chars: Vec<char> = s.chars().collect();
    if chars.len() <= max {
        s.to_string()
    } else {
        format!(
            "{}…",
            chars[..max.saturating_sub(1)].iter().collect::<String>()
        )
    }
}
