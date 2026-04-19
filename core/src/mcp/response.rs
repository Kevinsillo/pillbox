use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use validator::Validate;

// ─── Response ─────────────────────────────────────────────────────────────────

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
    pub fn ok(data: impl Serialize) -> Self {
        Self {
            ok: true,
            data: serde_json::to_value(data).ok(),
            error: None,
            message: None,
        }
    }

    pub fn err(error: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            ok: false,
            data: None,
            error: Some(error.into()),
            message: Some(message.into()),
        }
    }

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

pub fn not_found(entity: &str, id: impl std::fmt::Display) -> Response {
    Response::err("not_found", format!("{} {} no encontrada", entity, id))
}

pub fn from_value<T: for<'de> Deserialize<'de>>(v: Value) -> Result<T, Response> {
    serde_json::from_value(v).map_err(|e| Response::err("invalid_input", e.to_string()))
}

pub fn validate_input<T: Validate>(v: &T) -> Result<(), Response> {
    v.validate()
        .map_err(|e| Response::err("validation_error", e.to_string()))
}

pub fn anyhow_to_response(e: anyhow::Error) -> Response {
    let msg = e.to_string();
    if let Some(colon) = msg.find(':') {
        let code = msg[..colon].trim();
        if code.chars().all(|c| c.is_alphanumeric() || c == '_') {
            let message = msg[colon + 1..].trim().to_string();
            return Response::err(code, message);
        }
    }
    Response::err("error", msg)
}

// Alias para evitar importar Connection en todos los handlers
pub type Conn = Connection;
