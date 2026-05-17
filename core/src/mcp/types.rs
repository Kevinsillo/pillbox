//! Tipos del protocolo MCP persistente (NDJSON line-delimited).
//!
//! El cliente TS escribe un `Request` por línea en stdin; el servidor Rust
//! responde con un `Response` por línea en stdout. El `id` se propaga de
//! request a respuesta para que el cliente correlacione respuestas en flight.
//!
//! El campo `cwd` es OBLIGATORIO en cada Request — el binario MCP server es
//! singleton y no puede confiar en `std::env::current_dir()` para resolver
//! la DB local; el cwd lo aporta el cliente TS desde `process.cwd()`.

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Petición MCP recibida del cliente por stdin.
#[derive(Deserialize, Debug)]
pub struct Request {
    /// Identificador opaco generado por el cliente — se devuelve verbatim
    /// en la respuesta para que el cliente correlacione multiplexación.
    pub id: String,
    /// Nombre de la herramienta a invocar.
    pub tool: String,
    /// Input crudo de la herramienta (cada handler lo deserializa a su tipo).
    pub input: Value,
    /// Directorio de trabajo del cliente — usado para resolver la DB local
    /// (`{cwd}/.pillbox/pillbox.db`). Obligatorio: si falta, la línea se
    /// rechaza como `parse_error`.
    pub cwd: String,
}

/// Respuesta MCP que el servidor escribe en stdout (una línea JSON).
#[derive(Serialize, Debug)]
pub struct Response {
    /// Eco del `id` de la request original. Si la request no se pudo parsear
    /// (parse_error), el servidor responde con `id: ""`.
    pub id: String,
    pub ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}
