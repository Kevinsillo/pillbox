use serde::{Deserialize, Serialize};
use validator::Validate;

/// Input para abrir una prescription.
#[derive(Debug, Serialize, Deserialize, Validate)]
pub struct NewPrescription {
    /// FK a `bottles.id` (UUID).
    pub bottle_id: String,

    /// Título de la tarea/funcionalidad/bug que se va a trabajar.
    #[validate(length(min = 1, max = 255))]
    pub title: String,
}

/// Prescription tal como se devuelve al leerla.
#[derive(Debug, Serialize, Deserialize)]
pub struct Prescription {
    pub id: String,
    pub bottle_id: String,
    pub title: String,
    pub started_at: String,
    pub ended_at: Option<String>,
    pub deleted_at: Option<String>,
}
