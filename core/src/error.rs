use serde::Serialize;
use thiserror::Error;

#[derive(Debug, Error, Serialize)]
#[serde(tag = "error", rename_all = "snake_case")]
pub enum PillboxError {
    // ── Pill ──────────────────────────────────────────────────────────────────
    #[error("pill_not_found: no existe la pill con id={id}")]
    PillNotFound { id: i64 },

    // ── Capsule ───────────────────────────────────────────────────────────────
    #[error("capsule_not_found: no existe la capsule con id={id}")]
    CapsuleNotFound { id: i64 },
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

impl PillboxError {
    /// Código de error estable para clientes MCP/HTTP — sin parsear strings.
    pub fn code(&self) -> &'static str {
        match self {
            Self::PillNotFound { .. }                  => "pill_not_found",
            Self::CapsuleNotFound { .. }               => "capsule_not_found",
            Self::PrescriptionRequired { .. }          => "prescription_required",
            Self::PrescriptionAlreadyOpen { .. }       => "prescription_already_open",
            Self::PrescriptionNotFoundOrClosed { .. }  => "prescription_not_found_or_closed",
            Self::PrescriptionNotFound { .. }          => "prescription_not_found",
            Self::BottleNotFound { .. }                => "bottle_not_found",
            Self::BottleAlreadyExists { .. }           => "bottle_already_exists",
        }
    }
}
