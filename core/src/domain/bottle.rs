use serde::{Deserialize, Serialize};
use validator::Validate;

/// Input para crear un bottle (desde `pillbox bottle init`).
#[derive(Debug, Serialize, Deserialize, Validate)]
pub struct NewBottle {
    /// Slug generado automáticamente del nombre de carpeta. No lo elige el usuario.
    #[validate(length(min = 1, max = 255))]
    pub name: String,

    /// Nombre legible elegido por el usuario en el wizard de init.
    #[validate(length(min = 1, max = 255))]
    pub display_name: String,

    /// Ruta absoluta al directorio del proyecto.
    #[validate(length(min = 1))]
    pub directory: String,

    /// Scope de la DB: 'local' (.pillbox/pillbox.db) o 'global' (~/.pillbox/pillbox.db).
    pub scope: BottleScope,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BottleScope {
    Local,
    Global,
}

impl BottleScope {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Local => "local",
            Self::Global => "global",
        }
    }
}

impl std::fmt::Display for BottleScope {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

fn default_true() -> bool { true }

/// Bottle tal como se devuelve al listar.
#[derive(Debug, Serialize, Deserialize)]
pub struct Bottle {
    pub id: String,
    pub name: String,
    pub display_name: String,
    pub directory: String,
    pub scope: String,
    pub created_at: String,
    pub last_seen_at: String,
    /// false cuando el bottle está registrado globalmente pero su DB ya no existe en disco.
    #[serde(default = "default_true")]
    pub linked: bool,
    /// ID del registro en `registered_bottles` (presente en bottles con DB registrada).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reg_id: Option<i64>,
}
