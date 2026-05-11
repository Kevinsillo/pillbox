//! Tipos de dominio para búsqueda FTS5 en pills y capsules.

use serde::{Deserialize, Serialize};

/// Parámetros para búsqueda FTS5.
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct SearchParams {
    pub query: String,

    /// Filtrar por bottle (opcional). Útil en la DB global que agrega múltiples bottles.
    pub bottle_id: Option<String>,

    /// Filtrar por compound (opcional).
    pub compound: Option<String>,

    /// Número máximo de resultados. Default: 20.
    pub limit: Option<u32>,
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
