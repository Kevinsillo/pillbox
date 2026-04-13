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
    Config,
    Discovery,
    Learning,
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
            Self::Config              => "config",
            Self::Discovery           => "discovery",
            Self::Learning            => "learning",
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

/// Input para crear o actualizar una pill (validado antes de llegar al store).
#[derive(Debug, Serialize, Deserialize, Validate)]
pub struct NewPill {
    #[validate(length(min = 1, max = 255))]
    pub title: String,

    #[validate(length(min = 1, max = 5000))]
    pub content: String,

    pub compound: PillCompound,

    /// Nombre del bottle — se normaliza antes de llegar aquí.
    #[validate(length(min = 1, max = 255))]
    pub bottle: String,

    /// ID de la prescription activa (UUID v7).
    #[validate(length(min = 1))]
    pub prescription_id: String,

    /// Clave de identidad de la pill. Solo `[a-z0-9/-]`.
    #[validate(length(max = 120), custom(function = "validate_formula"))]
    pub formula: Option<String>,

    /// Quién dispensó la pill. FK a `dispenser_types.id`.
    pub dispenser: Option<String>,
}

/// Proyección reducida para listados y resultados de búsqueda.
#[derive(Debug, Serialize, Deserialize)]
pub struct PillSummary {
    pub id: i64,
    pub sync_id: String,
    pub compound: String,
    pub title: String,
    pub bottle: String,
    pub formula: Option<String>,
    pub dosage: i64,
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
    pub bottle: String,
    pub prescription_id: String,
    pub dispenser: Option<String>,
    pub formula: Option<String>,
    pub dosage: i64,
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

/// Valida que una formula use solo caracteres permitidos: `[a-z0-9/-]`.
fn validate_formula(formula: &str) -> Result<(), validator::ValidationError> {
    if formula
        .chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || "/-".contains(c))
    {
        Ok(())
    } else {
        Err(validator::ValidationError::new("formula_invalid_chars"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use validator::Validate;

    #[test]
    fn compound_as_str_roundtrip() {
        assert_eq!(PillCompound::Decision.as_str(), "decision");
        assert_eq!(PillCompound::PrescriptionSummary.as_str(), "prescription_summary");
    }

    #[test]
    fn new_pill_valid() {
        let pill = NewPill {
            title: "Test pill".into(),
            content: "Contenido de prueba".into(),
            compound: PillCompound::Decision,
            bottle: "my-project".into(),
            prescription_id: "01jq0000000000000000000000".into(),
            formula: Some("architecture/auth-flow".into()),
            dispenser: None,
        };
        assert!(pill.validate().is_ok());
    }

    #[test]
    fn formula_rejects_uppercase() {
        let pill = NewPill {
            title: "Test".into(),
            content: "Content".into(),
            compound: PillCompound::Manual,
            bottle: "proj".into(),
            prescription_id: "abc".into(),
            formula: Some("Architecture/Auth".into()),
            dispenser: None,
        };
        assert!(pill.validate().is_err());
    }

    #[test]
    fn formula_rejects_spaces() {
        let pill = NewPill {
            title: "T".into(),
            content: "C".into(),
            compound: PillCompound::Manual,
            bottle: "p".into(),
            prescription_id: "x".into(),
            formula: Some("has space".into()),
            dispenser: None,
        };
        assert!(pill.validate().is_err());
    }

    #[test]
    fn formula_rejects_underscore_and_dot() {
        for f in &["auth_flow", "api.version"] {
            let pill = NewPill {
                title: "T".into(),
                content: "C".into(),
                compound: PillCompound::Manual,
                bottle: "p".into(),
                prescription_id: "x".into(),
                formula: Some(f.to_string()),
                dispenser: None,
            };
            assert!(pill.validate().is_err(), "debería rechazar: {}", f);
        }
    }
}
