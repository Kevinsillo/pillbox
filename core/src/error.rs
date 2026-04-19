use serde::Serialize;
use thiserror::Error;

#[derive(Debug, Error, Serialize)]
#[serde(tag = "error", rename_all = "snake_case")]
pub enum PillboxError {
    // ── Prescription ──────────────────────────────────────────────────────────
    #[error("prescription_required: la prescription '{prescription_id}' no existe o está cerrada")]
    PrescriptionRequired { prescription_id: String },

    #[error(
        "prescription_already_open: '{title}' (id={id}, iniciada={started_at}, {pill_count} pills)"
    )]
    PrescriptionAlreadyOpen {
        id: String,
        title: String,
        started_at: String,
        pill_count: i64,
    },

    #[error("prescription_not_found_or_closed: {id}")]
    PrescriptionNotFoundOrClosed { id: String },

    #[error("prescription_not_found: {id}")]
    PrescriptionNotFound { id: String },

    // ── Bottle ────────────────────────────────────────────────────────────────
    #[error("bottle_not_found: no existe el bottle {bottle_id}")]
    BottleNotFound { bottle_id: i64 },

    #[error("bottle_already_exists: ya existe un bottle con el nombre '{name}'")]
    BottleAlreadyExists { name: String },
}
