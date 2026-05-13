//! Typed domain errors for Pillbox.
//!
//! [`PillboxError`] is the single business-error type. Infrastructure errors
//! (SQLite, I/O, network) propagate as [`anyhow::Error`].

use serde::Serialize;
use thiserror::Error;

/// Domain errors returned by Pillbox operations.
///
/// Serializable as JSON via `serde` for MCP/HTTP responses. Each variant
/// carries the fields a client needs to render a useful message without
/// parsing strings.
#[derive(Debug, Error, Serialize)]
#[serde(tag = "error", rename_all = "snake_case")]
pub enum PillboxError {
    // ── Pill ──────────────────────────────────────────────────────────────────
    #[error("pill_not_found: no pill exists with id={id}")]
    PillNotFound { id: String },

    // ── Capsule ───────────────────────────────────────────────────────────────
    #[error("capsule_not_found: no capsule exists with id={id}")]
    CapsuleNotFound { id: String },
    // ── Prescription ──────────────────────────────────────────────────────────
    #[error("prescription_required: prescription '{prescription_id}' does not exist or is closed")]
    PrescriptionRequired { prescription_id: String },

    #[error(
        "prescription_already_open: '{title}' (id={id}, started={started_at}, {pill_count} pills)"
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

    #[error("prescription_closed: prescription '{prescription_id}' is closed; reopen it before editing pills")]
    PrescriptionClosed { prescription_id: String },

    #[error(
        "prescription_collision: bottle '{bottle_id}' already has an open prescription (id={existing_id})"
    )]
    PrescriptionAlreadyOpenInBottle {
        bottle_id: String,
        existing_id: String,
    },

    // ── Bottle ────────────────────────────────────────────────────────────────
    #[error("bottle_not_found: no bottle exists with id={bottle_id}")]
    BottleNotFound { bottle_id: String },

    #[error("bottle_already_exists: a bottle named '{name}' already exists")]
    BottleAlreadyExists { name: String },

    // ── ID resolution ─────────────────────────────────────────────────────────
    #[error("invalid_id: '{id}' is too short (minimum 8 characters)")]
    InvalidId { id: String },

    #[error("ambiguous_id: prefix '{id_prefix}' matches {} records", candidates.len())]
    AmbiguousId {
        id_prefix: String,
        candidates: Vec<String>,
    },

    // ── Content ───────────────────────────────────────────────────────────────
    #[error("content_too_large: {actual} characters exceed the limit of {limit}")]
    ContentTooLarge { actual: usize, limit: usize },
}

impl PillboxError {
    /// Código de error estable para clientes MCP/HTTP — sin parsear strings.
    pub fn code(&self) -> &'static str {
        match self {
            Self::PillNotFound { .. } => "pill_not_found",
            Self::CapsuleNotFound { .. } => "capsule_not_found",
            Self::PrescriptionRequired { .. } => "prescription_required",
            Self::PrescriptionAlreadyOpen { .. } => "prescription_already_open",
            Self::PrescriptionNotFoundOrClosed { .. } => "prescription_not_found_or_closed",
            Self::PrescriptionNotFound { .. } => "prescription_not_found",
            Self::PrescriptionClosed { .. } => "prescription_closed",
            Self::PrescriptionAlreadyOpenInBottle { .. } => "prescription_collision",
            Self::BottleNotFound { .. } => "bottle_not_found",
            Self::BottleAlreadyExists { .. } => "bottle_already_exists",
            Self::InvalidId { .. } => "invalid_id",
            Self::AmbiguousId { .. } => "ambiguous_id",
            Self::ContentTooLarge { .. } => "content_too_large",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn code_matches_each_variant() {
        assert_eq!(
            PillboxError::PillNotFound { id: "1".into() }.code(),
            "pill_not_found"
        );
        assert_eq!(
            PillboxError::CapsuleNotFound { id: "2".into() }.code(),
            "capsule_not_found"
        );
        assert_eq!(
            PillboxError::PrescriptionRequired {
                prescription_id: "x".into()
            }
            .code(),
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
        assert_eq!(
            PillboxError::PrescriptionClosed {
                prescription_id: "x".into()
            }
            .code(),
            "prescription_closed"
        );
        assert_eq!(
            PillboxError::PrescriptionAlreadyOpenInBottle {
                bottle_id: "b".into(),
                existing_id: "rx".into(),
            }
            .code(),
            "prescription_collision"
        );
        assert_eq!(
            PillboxError::BottleNotFound {
                bottle_id: "uuid-test".into()
            }
            .code(),
            "bottle_not_found"
        );
        assert_eq!(
            PillboxError::BottleAlreadyExists { name: "n".into() }.code(),
            "bottle_already_exists"
        );
        assert_eq!(PillboxError::InvalidId { id: "abc".into() }.code(), "invalid_id");
        assert_eq!(
            PillboxError::AmbiguousId { id_prefix: "abc".into(), candidates: vec![] }.code(),
            "ambiguous_id"
        );
        assert_eq!(
            PillboxError::ContentTooLarge { actual: 6000, limit: 5000 }.code(),
            "content_too_large"
        );
    }

    #[test]
    fn display_contains_code() {
        let err = PillboxError::PillNotFound { id: "42".into() };
        assert!(err.to_string().contains("pill_not_found"));
        assert!(err.to_string().contains("42"));

        let err = PillboxError::BottleAlreadyExists {
            name: "my-proj".into(),
        };
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
        let err: Box<dyn std::error::Error> = Box::new(PillboxError::PillNotFound { id: "1".into() });
        assert!(err.to_string().contains("pill_not_found"));
    }
}
