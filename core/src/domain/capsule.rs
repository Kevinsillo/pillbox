//! Tipos de dominio para capsules — conocimiento personal global sin bottle.

use serde::{Deserialize, Serialize};
use validator::Validate;

/// Input para crear una capsule.
#[derive(Debug, Serialize, Deserialize, Validate)]
pub struct NewCapsule {
    #[validate(length(min = 1, max = 255))]
    pub title: String,

    #[validate(length(min = 1, max = 5000))]
    pub content: String,

    #[validate(length(min = 1, max = 64))]
    pub compound: String,
}

/// Capsule completa.
#[derive(Debug, Serialize, Deserialize)]
pub struct Capsule {
    pub id: String,
    pub compound: String,
    pub title: String,
    pub content: String,
    pub created_at: String,
    pub updated_at: String,
    pub deleted_at: Option<String>,
}

/// Campos actualizables en una revisión parcial.
#[derive(Debug, Default, Serialize, Deserialize, Validate)]
pub struct CapsulePatch {
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
    fn new_capsule_valid() {
        let cap = NewCapsule {
            title: "Test capsule".into(),
            content: "Contenido válido".into(),
            compound: "convention".into(),
        };
        assert!(cap.validate().is_ok());
    }

    #[test]
    fn new_capsule_empty_title_fails_validation() {
        let cap = NewCapsule {
            title: "".into(),
            content: "Contenido válido".into(),
            compound: "convention".into(),
        };
        assert!(cap.validate().is_err());
    }

    #[test]
    fn new_capsule_content_too_long_fails_validation() {
        let cap = NewCapsule {
            title: "Título".into(),
            content: "x".repeat(5001),
            compound: "goal".into(),
        };
        assert!(cap.validate().is_err());
    }

    #[test]
    fn new_capsule_compound_too_long_fails_validation() {
        let cap = NewCapsule {
            title: "Título".into(),
            content: "Contenido".into(),
            compound: "x".repeat(65),
        };
        assert!(cap.validate().is_err());
    }

    #[test]
    fn capsule_patch_empty_title_fails_validation() {
        let patch = CapsulePatch {
            title: Some("".into()),
            content: None,
            compound: None,
        };
        assert!(patch.validate().is_err());
    }
}
