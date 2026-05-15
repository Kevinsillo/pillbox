//! Tipos de respuesta MCP y helpers compartidos por todos los handlers.

use pillbox::error::{ContentOp, PillboxError};
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use validator::Validate;

// ─── Response ─────────────────────────────────────────────────────────────────

/// Respuesta JSON uniforme para todas las herramientas MCP.
#[derive(Serialize)]
pub struct Response {
    pub ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

impl Response {
    /// Construye una respuesta de éxito con `ok: true` y los datos serializados.
    pub fn ok(data: impl Serialize) -> Self {
        Self {
            ok: true,
            data: serde_json::to_value(data).ok(),
            error: None,
            message: None,
        }
    }

    /// Construye una respuesta de error con `ok: false`, código y mensaje.
    pub fn err(error: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            ok: false,
            data: None,
            error: Some(error.into()),
            message: Some(message.into()),
        }
    }

    /// Construye una respuesta de error con datos adicionales (ej: prescription existente).
    pub fn err_with_data(
        error: impl Into<String>,
        message: impl Into<String>,
        data: Value,
    ) -> Self {
        Self {
            ok: false,
            data: Some(data),
            error: Some(error.into()),
            message: Some(message.into()),
        }
    }
}

// ─── Helpers compartidos por handlers ─────────────────────────────────────────

/// Deserializa un [`Value`] JSON al tipo `T`, devolviendo [`Response::err`] si falla.
pub fn from_value<T: for<'de> Deserialize<'de>>(v: Value) -> Result<T, Response> {
    serde_json::from_value(v).map_err(|e| Response::err("invalid_input", e.to_string()))
}

/// Valida una struct con `validator`, devolviendo [`Response::err`] si hay errores.
pub fn validate_input<T: Validate>(v: &T) -> Result<(), Response> {
    v.validate()
        .map_err(|e| Response::err("validation_error", e.to_string()))
}

/// Verifica que el contenido no supere el límite de caracteres.
///
/// `op` distingue create (`*_store`) de update (`*_revise`) para que el
/// cliente pueda recomendar la remediación correcta (split vs trim).
///
/// Devuelve [`Response`] con código `content_too_large` y datos estructurados
/// (`actual`, `limit`, `operation`) cuando el contenido excede el límite. El
/// formatter del cliente decide cómo presentar la información al modelo.
pub fn check_content_size(
    content: &str,
    limit: usize,
    op: ContentOp,
) -> Result<(), Response> {
    let actual = content.chars().count();
    if actual <= limit {
        return Ok(());
    }
    Err(from_pillbox(&PillboxError::ContentTooLarge {
        actual,
        limit,
        operation: op,
    }))
}

/// Convierte un [`anyhow::Error`] en [`Response`], usando el código tipado si es [`PillboxError`].
pub fn anyhow_to_response(e: anyhow::Error) -> Response {
    if let Some(pe) = e.downcast_ref::<PillboxError>() {
        return from_pillbox(pe);
    }
    Response::err("internal_error", e.to_string())
}

/// Convierte un [`PillboxError`] tipado en [`Response`], incluyendo datos adicionales
/// para la variante `PrescriptionAlreadyOpen`.
pub fn from_pillbox(pe: &PillboxError) -> Response {
    match pe {
        PillboxError::PrescriptionAlreadyOpen {
            id,
            title,
            started_at,
            pill_count,
        } => Response::err_with_data(
            pe.code(),
            pe.to_string(),
            json!({
                "id": id,
                "title": title,
                "started_at": started_at,
                "pill_count": pill_count,
            }),
        ),
        PillboxError::PrescriptionClosed { prescription_id } => Response::err_with_data(
            pe.code(),
            pe.to_string(),
            json!({ "prescription_id": prescription_id }),
        ),
        PillboxError::PrescriptionAlreadyOpenInBottle {
            bottle_id,
            existing_id,
        } => Response::err_with_data(
            pe.code(),
            pe.to_string(),
            json!({ "bottle_id": bottle_id, "existing_id": existing_id }),
        ),
        PillboxError::AmbiguousId { id_prefix, candidates } => Response::err_with_data(
            pe.code(),
            pe.to_string(),
            json!({ "id_prefix": id_prefix, "candidates": candidates }),
        ),
        PillboxError::InvalidId { id } => Response::err_with_data(
            pe.code(),
            pe.to_string(),
            json!({ "id": id }),
        ),
        PillboxError::ContentTooLarge { actual, limit, operation } => Response::err_with_data(
            pe.code(),
            pe.to_string(),
            json!({ "actual": actual, "limit": limit, "operation": operation }),
        ),
        _ => Response::err(pe.code(), pe.to_string()),
    }
}

/// Alias de [`rusqlite::Connection`] para simplificar las firmas de los handlers.
pub type Conn = Connection;
