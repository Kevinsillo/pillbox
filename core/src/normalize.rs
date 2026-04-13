use unicode_normalization::UnicodeNormalization;

/// Normaliza el nombre de un bottle para usarlo como clave en DB.
///
/// Reglas:
/// - Lowercase
/// - Trim de espacios en extremos
/// - Colapsa espacios, guiones y underscores múltiples en un solo guión
///
/// El nombre original (display_name) se preserva por separado.
pub fn bottle_name(raw: &str) -> String {
    let lower = raw.trim().to_lowercase();

    // Normaliza unicode NFC para consistencia
    let nfc: String = lower.nfc().collect();

    // Reemplaza caracteres no alfanuméricos (excepto `-`) por guión
    let mut result = String::with_capacity(nfc.len());
    let mut prev_dash = false;

    for c in nfc.chars() {
        if c.is_alphanumeric() {
            result.push(c);
            prev_dash = false;
        } else if !prev_dash {
            result.push('-');
            prev_dash = true;
        }
    }

    // Trim de guiones en extremos
    result.trim_matches('-').to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bottle_name_lowercases_and_trims() {
        assert_eq!(bottle_name("  My Project  "), "my-project");
    }

    #[test]
    fn bottle_name_collapses_separators() {
        assert_eq!(bottle_name("my--project__name"), "my-project-name");
    }

    #[test]
    fn bottle_name_preserves_unicode_letters() {
        let result = bottle_name("Mi Proyecto");
        assert_eq!(result, "mi-proyecto");
    }
}
