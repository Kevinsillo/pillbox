//! Tipos genéricos de paginación reutilizables para listados y búsquedas.
//!
//! `PaginationParams` se deserializa desde query strings HTTP o argumentos MCP;
//! `Paginated<T>` envuelve respuestas con metadatos (`total`, `page`, `page_size`).

use serde::{Deserialize, Deserializer, Serialize};

fn default_page() -> u32 {
    1
}

fn default_page_size() -> u32 {
    20
}

/// Deserializa un `u32` aceptando tanto entero como string.
///
/// Necesario para `#[serde(flatten)]` con axum/serde_urlencoded, que entrega
/// los valores de query string como strings y `flatten` desactiva la coerción
/// automática.
fn u32_from_str_or_int<'de, D>(deserializer: D) -> Result<u32, D::Error>
where
    D: Deserializer<'de>,
{
    use serde::de::Error;
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum StringOrInt<'a> {
        String(&'a str),
        Int(u32),
    }
    match StringOrInt::deserialize(deserializer)? {
        StringOrInt::Int(n) => Ok(n),
        StringOrInt::String(s) => s.parse::<u32>().map_err(D::Error::custom),
    }
}

/// Parámetros de paginación 1-indexed.
///
/// `page` arranca en 1. `page_size` está acotado a `1..=100` por `validate`.
#[derive(Debug, Clone, Deserialize)]
pub struct PaginationParams {
    #[serde(default = "default_page", deserialize_with = "u32_from_str_or_int")]
    pub page: u32,

    #[serde(
        default = "default_page_size",
        deserialize_with = "u32_from_str_or_int"
    )]
    pub page_size: u32,
}

impl Default for PaginationParams {
    fn default() -> Self {
        Self {
            page: 1,
            page_size: 20,
        }
    }
}

impl PaginationParams {
    /// Valida que `page >= 1` y `page_size` esté en `1..=100`.
    pub fn validate(&self) -> Result<(), String> {
        if self.page == 0 {
            return Err("page must be >= 1".into());
        }
        if self.page_size == 0 || self.page_size > 100 {
            return Err("page_size must be 1..=100".into());
        }
        Ok(())
    }

    /// Offset SQL derivado: `(page - 1) * page_size`.
    pub fn offset(&self) -> u32 {
        (self.page - 1) * self.page_size
    }

    /// Límite SQL derivado: `page_size`.
    pub fn limit(&self) -> u32 {
        self.page_size
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_rejects_page_zero() {
        let p = PaginationParams {
            page: 0,
            page_size: 20,
        };
        assert!(p.validate().is_err());
    }

    #[test]
    fn validate_rejects_page_size_zero() {
        let p = PaginationParams {
            page: 1,
            page_size: 0,
        };
        assert!(p.validate().is_err());
    }

    #[test]
    fn validate_rejects_page_size_over_100() {
        let p = PaginationParams {
            page: 1,
            page_size: 101,
        };
        assert!(p.validate().is_err());
    }

    #[test]
    fn validate_accepts_page_one_page_size_100() {
        let p = PaginationParams {
            page: 1,
            page_size: 100,
        };
        assert!(p.validate().is_ok());
    }

    #[test]
    fn offset_computed_correctly() {
        let p = PaginationParams {
            page: 3,
            page_size: 20,
        };
        assert_eq!(p.offset(), 40);
    }

    #[test]
    fn default_is_page_one_page_size_20() {
        let p = PaginationParams::default();
        assert_eq!(p.page, 1);
        assert_eq!(p.page_size, 20);
    }
}

/// Envoltorio genérico de respuesta paginada.
#[derive(Debug, Serialize)]
pub struct Paginated<T: Serialize> {
    pub items: Vec<T>,
    pub total: u64,
    pub page: u32,
    pub page_size: u32,
    /// Indica si los resultados provienen de la pasada fuzzy (Jaro-Winkler).
    /// Solo aplica a búsquedas FTS; en otros listados es `false`.
    #[serde(default)]
    pub used_fuzzy: bool,
}
