use serde::{Deserialize, Serialize};

/// Bottle tal como se devuelve al listar.
#[derive(Debug, Serialize, Deserialize)]
pub struct Bottle {
    pub id: i64,
    pub name: String,
    pub display_name: String,
    pub db_scope: String,
    pub created_at: String,
    pub last_seen_at: String,
}
