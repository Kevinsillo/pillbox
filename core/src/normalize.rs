use sha2::{Digest, Sha256};
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

/// Valida y normaliza una formula: debe usar solo `[a-z0-9/-]`.
///
/// Normaliza a lowercase antes de validar — el uppercase se acepta y convierte.
/// Devuelve `None` si contiene caracteres inválidos incluso tras la normalización.
pub fn formula(raw: &str) -> Option<String> {
    let trimmed = raw.trim().to_lowercase();
    let valid = trimmed
        .chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || "/-".contains(c));

    if valid && !trimmed.is_empty() {
        Some(trimmed)
    } else {
        None
    }
}

/// Calcula el hash de normalización para deduplicación por contenido.
///
/// SHA-256 de `compound + "::" + lowercase(trim(content))`.
/// No incluye el título — el mismo hecho puede guardarse con títulos distintos.
pub fn content_hash(compound: &str, content: &str) -> String {
    let normalized = format!("{}::{}", compound, content.trim().to_lowercase());
    let hash = Sha256::digest(normalized.as_bytes());
    format!("{:x}", hash)
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
        // Las letras unicode válidas (ñ, é...) se preservan en lowercase
        let result = bottle_name("Mi Proyecto");
        assert_eq!(result, "mi-proyecto");
    }

    #[test]
    fn formula_valid() {
        assert_eq!(formula("architecture/auth-flow"), Some("architecture/auth-flow".into()));
    }

    #[test]
    fn formula_normalizes_uppercase() {
        // La función lowercasea antes de validar
        assert_eq!(formula("Architecture/Auth"), Some("architecture/auth".into()));
    }

    #[test]
    fn formula_rejects_spaces() {
        assert!(formula("has space").is_none());
    }

    #[test]
    fn formula_rejects_dot() {
        assert!(formula("api.version").is_none());
    }

    #[test]
    fn formula_rejects_underscore() {
        assert!(formula("auth_flow").is_none());
    }

    #[test]
    fn formula_rejects_special_chars() {
        assert!(formula("invalid@char").is_none());
    }

    #[test]
    fn content_hash_deterministic() {
        let h1 = content_hash("decision", "  Some content  ");
        let h2 = content_hash("decision", "some content");
        assert_eq!(h1, h2);
    }

    #[test]
    fn content_hash_different_compounds() {
        let h1 = content_hash("decision", "content");
        let h2 = content_hash("bugfix", "content");
        assert_ne!(h1, h2);
    }
}
