use serde::{Deserialize, Serialize};

/// Parámetros para búsqueda FTS5.
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct SearchParams {
    pub query: String,

    /// Filtrar por bottle (opcional).
    pub bottle: Option<String>,

    /// Filtrar por compound (opcional).
    pub compound: Option<String>,

    /// Número máximo de resultados. Default: 20.
    pub limit: Option<u32>,
}

/// Resultado individual de una búsqueda.
#[derive(Debug, Serialize, Deserialize)]
pub struct SearchResult {
    pub id: i64,
    pub sync_id: String,
    pub compound: String,
    pub title: String,
    /// Snippet del contenido con el match resaltado.
    pub snippet: String,
    pub bottle: Option<String>,
    pub formula: Option<String>,
    pub updated_at: String,
    /// Rank FTS5 (negativo — más cercano a 0 = más relevante).
    pub rank: f64,
}
