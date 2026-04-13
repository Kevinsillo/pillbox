use serde::{Deserialize, Serialize};
use validator::Validate;

/// Input para abrir una prescription.
#[derive(Debug, Serialize, Deserialize, Validate)]
pub struct NewPrescription {
    #[validate(length(min = 1, max = 255))]
    pub bottle: String,

    /// Directorio de trabajo al abrir la prescription.
    #[validate(length(min = 1))]
    pub directory: String,
}

/// Prescription tal como se devuelve al leerla.
#[derive(Debug, Serialize, Deserialize)]
pub struct Prescription {
    pub id: String,
    pub bottle: String,
    pub directory: String,
    pub started_at: String,
    pub ended_at: Option<String>,
    pub summary: Option<String>,
}
