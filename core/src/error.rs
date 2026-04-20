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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn code_matches_each_variant() {
        assert_eq!(PillboxError::PillNotFound { id: 1 }.code(), "pill_not_found");
        assert_eq!(PillboxError::CapsuleNotFound { id: 2 }.code(), "capsule_not_found");
        assert_eq!(
            PillboxError::PrescriptionRequired { prescription_id: "x".into() }.code(),
            "prescription_required"
        );
        assert_eq!(
            PillboxError::PrescriptionAlreadyOpen {
                id: "x".into(),
                title: "t".into(),
                started_at: "2026-01-01".into(),
                pill_count: 0,
            }
            .code(),
            "prescription_already_open"
        );
        assert_eq!(
            PillboxError::PrescriptionNotFoundOrClosed { id: "x".into() }.code(),
            "prescription_not_found_or_closed"
        );
        assert_eq!(
            PillboxError::PrescriptionNotFound { id: "x".into() }.code(),
            "prescription_not_found"
        );
        assert_eq!(PillboxError::BottleNotFound { bottle_id: 5 }.code(), "bottle_not_found");
        assert_eq!(
            PillboxError::BottleAlreadyExists { name: "n".into() }.code(),
            "bottle_already_exists"
        );
    }

    #[test]
    fn display_contains_code() {
        let err = PillboxError::PillNotFound { id: 42 };
        assert!(err.to_string().contains("pill_not_found"));
        assert!(err.to_string().contains("42"));

        let err = PillboxError::BottleAlreadyExists { name: "my-proj".into() };
        assert!(err.to_string().contains("bottle_already_exists"));
        assert!(err.to_string().contains("my-proj"));

        let err = PillboxError::PrescriptionAlreadyOpen {
            id: "abc".into(),
            title: "Mi sesión".into(),
            started_at: "2026-01-01".into(),
            pill_count: 3,
        };
        assert!(err.to_string().contains("prescription_already_open"));
        assert!(err.to_string().contains("Mi sesión"));
        assert!(err.to_string().contains('3'));
    }

    #[test]
    fn error_is_std_error() {
        let err: Box<dyn std::error::Error> =
            Box::new(PillboxError::PillNotFound { id: 1 });
        assert!(err.to_string().contains("pill_not_found"));
    }
}
