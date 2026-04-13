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
            Self::Decision            => "decision",
            Self::Architecture        => "architecture",
            Self::Bugfix              => "bugfix",
            Self::Pattern             => "pattern",
            Self::Discovery           => "discovery",
            Self::Learning            => "learning",
            Self::Feedback            => "feedback",
            Self::PrescriptionSummary => "prescription_summary",
            Self::Manual              => "manual",
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

    /// Quién dispensó la pill. FK a `dispenser_types.id`.
    pub dispenser: Option<String>,

    pub author_name: Option<String>,
    pub author_email: Option<String>,
}

/// Proyección reducida para listados y resultados de búsqueda.
#[derive(Debug, Serialize, Deserialize)]
pub struct PillSummary {
    pub id: i64,
    pub sync_id: String,
    pub compound: String,
    pub title: String,
    pub created_at: String,
    pub updated_at: String,
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
    pub dispenser: Option<String>,
    pub author_name: Option<String>,
    pub author_email: Option<String>,
    pub created_at: String,
    pub updated_at: String,
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
        assert_eq!(PillCompound::PrescriptionSummary.as_str(), "prescription_summary");
    }

    #[test]
    fn new_pill_valid() {
        let pill = NewPill {
            title: "Test pill".into(),
            content: "Contenido de prueba".into(),
            compound: PillCompound::Decision,
            prescription_id: "01jq0000000000000000000000".into(),
            dispenser: None,
            author_name: None,
            author_email: None,
        };
        assert!(pill.validate().is_ok());
    }
}
