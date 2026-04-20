use serde::{Deserialize, Serialize};
use validator::Validate;

/// Tipos de capsule. Los valores deben coincidir con los IDs en `capsule_compounds`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CapsuleCompound {
    Convention,
    Workflow,
    Environment,
    Context,
    Goal,
    Feedback,
    Manual,
}

impl CapsuleCompound {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Convention => "convention",
            Self::Workflow => "workflow",
            Self::Environment => "environment",
            Self::Context => "context",
            Self::Goal => "goal",
            Self::Feedback => "feedback",
            Self::Manual => "manual",
        }
    }
}

impl std::fmt::Display for CapsuleCompound {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Input para crear una capsule.
#[derive(Debug, Serialize, Deserialize, Validate)]
pub struct NewCapsule {
    #[validate(length(min = 1, max = 255))]
    pub title: String,

    #[validate(length(min = 1, max = 5000))]
    pub content: String,

    pub compound: CapsuleCompound,
}

/// Proyección reducida para listados.
#[derive(Debug, Serialize, Deserialize)]
pub struct CapsuleSummary {
    pub id: i64,
    pub sync_id: String,
    pub compound: String,
    pub title: String,
    pub created_at: String,
    pub updated_at: String,
}

/// Capsule completa.
#[derive(Debug, Serialize, Deserialize)]
pub struct Capsule {
    pub id: i64,
    pub sync_id: String,
    pub compound: String,
    pub title: String,
    pub content: String,
    pub created_at: String,
    pub updated_at: String,
}

/// Campos actualizables en una revisión parcial.
#[derive(Debug, Default, Serialize, Deserialize, Validate)]
pub struct CapsulePatch {
    #[validate(length(min = 1, max = 255))]
    pub title: Option<String>,

    #[validate(length(min = 1, max = 5000))]
    pub content: Option<String>,

    pub compound: Option<CapsuleCompound>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compound_as_str_roundtrip() {
        assert_eq!(CapsuleCompound::Convention.as_str(), "convention");
        assert_eq!(CapsuleCompound::Environment.as_str(), "environment");
        assert_eq!(CapsuleCompound::Feedback.as_str(), "feedback");
    }

    #[test]
    fn all_compounds_as_str() {
        assert_eq!(CapsuleCompound::Convention.as_str(), "convention");
        assert_eq!(CapsuleCompound::Workflow.as_str(), "workflow");
        assert_eq!(CapsuleCompound::Environment.as_str(), "environment");
        assert_eq!(CapsuleCompound::Context.as_str(), "context");
        assert_eq!(CapsuleCompound::Goal.as_str(), "goal");
        assert_eq!(CapsuleCompound::Feedback.as_str(), "feedback");
        assert_eq!(CapsuleCompound::Manual.as_str(), "manual");
    }

    #[test]
    fn compound_display_matches_as_str() {
        for c in [
            CapsuleCompound::Convention,
            CapsuleCompound::Workflow,
            CapsuleCompound::Environment,
            CapsuleCompound::Context,
            CapsuleCompound::Goal,
            CapsuleCompound::Feedback,
            CapsuleCompound::Manual,
        ] {
            assert_eq!(format!("{c}"), c.as_str());
        }
    }

    #[test]
    fn new_capsule_empty_title_fails_validation() {
        let cap = NewCapsule {
            title: "".into(),
            content: "Contenido válido".into(),
            compound: CapsuleCompound::Convention,
        };
        assert!(cap.validate().is_err());
    }

    #[test]
    fn new_capsule_content_too_long_fails_validation() {
        let cap = NewCapsule {
            title: "Título".into(),
            content: "x".repeat(5001),
            compound: CapsuleCompound::Goal,
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
