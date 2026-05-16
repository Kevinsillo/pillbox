//! Tipos de dominio para búsqueda FTS5 en pills y capsules.

use serde::{Deserialize, Deserializer, Serialize};

/// Deserializa un `bool` aceptando tanto booleano como string ("true"/"false"/"1"/"0").
///
/// Necesario para `#[serde(flatten)]` con axum/serde_urlencoded, que entrega
/// los valores de query string como strings y `flatten` desactiva la coerción
/// automática.
fn bool_from_str_or_bool<'de, D>(deserializer: D) -> Result<bool, D::Error>
where
    D: Deserializer<'de>,
{
    use serde::de::Error;
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum StringOrBool<'a> {
        Bool(bool),
        String(&'a str),
    }
    match StringOrBool::deserialize(deserializer)? {
        StringOrBool::Bool(b) => Ok(b),
        StringOrBool::String(s) => match s {
            "true" | "1" => Ok(true),
            "false" | "0" => Ok(false),
            other => Err(D::Error::custom(format!("invalid boolean: {other}"))),
        },
    }
}

/// Parámetros para búsqueda FTS5.
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct SearchParams {
    pub query: String,

    /// Filtrar por bottle (opcional). Útil en la DB global que agrega múltiples bottles.
    pub bottle_id: Option<String>,

    /// Filtrar por compound (opcional).
    pub compound: Option<String>,

    /// Activa expansión fuzzy (Jaro-Winkler) sobre los términos de la query.
    /// Default: false (prefix match estricto, sin tolerancia a typos).
    #[serde(default, deserialize_with = "bool_from_str_or_bool")]
    pub fuzzy: bool,
}

/// Resultado individual de una búsqueda.
#[derive(Debug, Serialize, Deserialize)]
pub struct SearchResult {
    pub id: String,
    pub compound: String,
    pub title: String,
    /// Snippet del contenido con el match resaltado.
    pub snippet: String,
    pub created_at: String,
    pub updated_at: String,
    /// Rank FTS5 (negativo — más cercano a 0 = más relevante).
    pub rank: f64,
    /// Solo presente en pills.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prescription_id: Option<String>,
    /// Solo presente en pills (a través del join con prescriptions).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bottle_id: Option<String>,
}
