use serde::{Deserialize, Serialize};
use validator::Validate;

/// Tipos de pill. Los valores deben coincidir con los IDs en `pill_compounds`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PillCompound {
    Decision,
    Architecture,
    Bugfix,
    Pattern,
    Discovery,
    Learning,
    Feedback,
    PrescriptionSummary,
    Manual,
}

impl PillCompound {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Decision => "decision",
            Self::Architecture => "architecture",
            Self::Bugfix => "bugfix",
            Self::Pattern => "pattern",
            Self::Discovery => "discovery",
            Self::Learning => "learning",
            Self::Feedback => "feedback",
            Self::PrescriptionSummary => "prescription_summary",
            Self::Manual => "manual",
        }
    }
}

impl std::fmt::Display for PillCompound {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Input para crear una pill.
#[derive(Debug, Serialize, Deserialize, Validate)]
pub struct NewPill {
    #[validate(length(min = 1, max = 255))]
    pub title: String,

    #[validate(length(min = 1, max = 5000))]
    pub content: String,

    pub compound: PillCompound,

    /// ID de la prescription activa (UUID v7).
    #[validate(length(min = 1))]
    pub prescription_id: String,

    pub author_name: Option<String>,
    pub author_email: Option<String>,
}

/// Pill completa tal como se devuelve al leerla por ID.
#[derive(Debug, Serialize, Deserialize)]
pub struct Pill {
    pub id: i64,
    pub sync_id: String,
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

    pub compound: Option<PillCompound>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compound_as_str_roundtrip() {
        assert_eq!(PillCompound::Decision.as_str(), "decision");
        assert_eq!(PillCompound::Feedback.as_str(), "feedback");
        assert_eq!(
            PillCompound::PrescriptionSummary.as_str(),
            "prescription_summary"
        );
    }

    #[test]
    fn new_pill_valid() {
        let pill = NewPill {
            title: "Test pill".into(),
            content: "Contenido de prueba".into(),
            compound: PillCompound::Decision,
            prescription_id: "01jq0000000000000000000000".into(),
            author_name: None,
            author_email: None,
        };
        assert!(pill.validate().is_ok());
    }

    #[test]
    fn all_compounds_as_str() {
        assert_eq!(PillCompound::Decision.as_str(), "decision");
        assert_eq!(PillCompound::Architecture.as_str(), "architecture");
        assert_eq!(PillCompound::Bugfix.as_str(), "bugfix");
        assert_eq!(PillCompound::Pattern.as_str(), "pattern");
        assert_eq!(PillCompound::Discovery.as_str(), "discovery");
        assert_eq!(PillCompound::Learning.as_str(), "learning");
        assert_eq!(PillCompound::Feedback.as_str(), "feedback");
        assert_eq!(
            PillCompound::PrescriptionSummary.as_str(),
            "prescription_summary"
        );
        assert_eq!(PillCompound::Manual.as_str(), "manual");
    }

    #[test]
    fn compound_display_matches_as_str() {
        for c in [
            PillCompound::Decision,
            PillCompound::Architecture,
            PillCompound::Bugfix,
            PillCompound::Pattern,
            PillCompound::Discovery,
            PillCompound::Learning,
            PillCompound::Feedback,
            PillCompound::PrescriptionSummary,
            PillCompound::Manual,
        ] {
            assert_eq!(format!("{c}"), c.as_str());
        }
    }

    #[test]
    fn new_pill_empty_title_fails_validation() {
        let pill = NewPill {
            title: "".into(),
            content: "Contenido".into(),
            compound: PillCompound::Decision,
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
            compound: PillCompound::Decision,
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
