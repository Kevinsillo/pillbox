use serde::{Deserialize, Serialize};
use validator::Validate;

/// Tipos de capsule. Los valores deben coincidir con los IDs en `capsule_compounds`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CapsuleCompound {
    Preference,
    Convention,
    Workflow,
    Skill,
    Context,
    Goal,
    Constraint,
    Manual,
}

impl CapsuleCompound {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Preference  => "preference",
            Self::Convention  => "convention",
            Self::Workflow    => "workflow",
            Self::Skill       => "skill",
            Self::Context     => "context",
            Self::Goal        => "goal",
            Self::Constraint  => "constraint",
            Self::Manual      => "manual",
        }
    }
}

impl std::fmt::Display for CapsuleCompound {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Input para crear o actualizar una capsule.
#[derive(Debug, Serialize, Deserialize, Validate)]
pub struct NewCapsule {
    #[validate(length(min = 1, max = 255))]
    pub title: String,

    #[validate(length(min = 1, max = 5000))]
    pub content: String,

    pub compound: CapsuleCompound,

    #[validate(length(max = 120), custom(function = "validate_formula"))]
    pub formula: Option<String>,
}

/// Proyección reducida para listados.
#[derive(Debug, Serialize, Deserialize)]
pub struct CapsuleSummary {
    pub id: i64,
    pub sync_id: String,
    pub compound: String,
    pub title: String,
    pub formula: Option<String>,
    pub dosage: i64,
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
    pub formula: Option<String>,
    pub dosage: i64,
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
