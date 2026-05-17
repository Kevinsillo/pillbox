//! Tipos que mapean directamente a filas de la base de datos.
//!
//! Sin lógica de negocio — son representaciones internas que usa la capa
//! de store para leer y escribir en SQLite.

/// Fila de la tabla `pills`.

#[derive(Debug, Clone)]
pub struct PillRow {
    pub id: i64,
    pub sync_id: String,
    pub compound: String,
    pub title: String,
    pub content: String,
    pub prescription_id: String,
    pub author_name: Option<String>,
    pub author_email: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub deleted_at: Option<String>,
    pub views: i64,
}

/// Fila de la tabla `capsules`.
#[derive(Debug, Clone)]
pub struct CapsuleRow {
    pub id: i64,
    pub sync_id: String,
    pub compound: String,
    pub title: String,
    pub content: String,
    pub created_at: String,
    pub updated_at: String,
    pub deleted_at: Option<String>,
    pub views: i64,
}

/// Fila de la tabla `prescriptions`.
#[derive(Debug, Clone)]
pub struct PrescriptionRow {
    pub id: String,
    pub bottle_id: i64,
    pub title: String,
    pub started_at: String,
    pub ended_at: Option<String>,
    pub deleted_at: Option<String>,
    pub views: i64,
}

/// Fila de la tabla `bottles`.
#[derive(Debug, Clone)]
pub struct BottleRow {
    pub id: i64,
    pub name: String,
    pub display_name: String,
    pub directory: String,
    pub scope: String,
    pub created_at: String,
    pub last_seen_at: String,
    pub views: i64,
}
