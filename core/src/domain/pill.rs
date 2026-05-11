//! Tipos de dominio para pills — unidades de conocimiento ligadas a una prescription.

use serde::{Deserialize, Serialize};
use validator::Validate;

/// Input para crear una pill.
#[derive(Debug, Serialize, Deserialize, Validate)]
pub struct NewPill {
    #[validate(length(min = 1, max = 255))]
    pub title: String,

    #[validate(length(min = 1, max = 5000))]
    pub content: String,

    #[validate(length(min = 1, max = 64))]
    pub compound: String,

    /// ID de la prescription activa (UUID v7).
    #[validate(length(min = 1))]
    pub prescription_id: String,

    pub author_name: Option<String>,
    pub author_email: Option<String>,
}

/// Pill completa tal como se devuelve al leerla por ID.
#[derive(Debug, Serialize, Deserialize)]
pub struct Pill {
    pub id: String,
    pub compound: String,
    pub title: String,
    pub content: String,
    pub prescription_id: String,
    pub author_name: Option<String>,
    pub author_email: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub deleted_at: Option<String>,
}

/// Campos actualizables en una revisión parcial.
#[derive(Debug, Default, Serialize, Deserialize, Validate)]
pub struct PillPatch {
    #[validate(length(min = 1, max = 255))]
    pub title: Option<String>,

    #[validate(length(min = 1, max = 5000))]
    pub content: Option<String>,

    #[validate(length(min = 1, max = 64))]
    pub compound: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_pill_valid() {
        let pill = NewPill {
            title: "Test pill".into(),
            content: "Contenido de prueba".into(),
            compound: "decision".into(),
            prescription_id: "01jq0000000000000000000000".into(),
            author_name: None,
            author_email: None,
        };
        assert!(pill.validate().is_ok());
    }

    #[test]
    fn new_pill_empty_title_fails_validation() {
        let pill = NewPill {
            title: "".into(),
            content: "Contenido".into(),
            compound: "decision".into(),
            prescription_id: "abc".into(),
            author_name: None,
            author_email: None,
        };
        assert!(pill.validate().is_err());
    }

    #[test]
    fn new_pill_content_too_long_fails_validation() {
        let pill = NewPill {
            title: "Título válido".into(),
            content: "x".repeat(5001),
            compound: "decision".into(),
            prescription_id: "abc".into(),
            author_name: None,
            author_email: None,
        };
        assert!(pill.validate().is_err());
    }

    #[test]
    fn new_pill_compound_too_long_fails_validation() {
        let pill = NewPill {
            title: "Título válido".into(),
            content: "Contenido".into(),
            compound: "x".repeat(65),
            prescription_id: "abc".into(),
            author_name: None,
            author_email: None,
        };
        assert!(pill.validate().is_err());
    }

    #[test]
    fn pill_patch_empty_title_fails_validation() {
        let patch = PillPatch {
            title: Some("".into()),
            content: None,
            compound: None,
        };
        assert!(patch.validate().is_err());
    }
}
