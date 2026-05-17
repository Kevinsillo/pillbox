//! Selector de esquema de base de datos.
//!
//! Indica qué SQL inicial debe ejecutar el runner de migraciones al abrir
//! una conexión nueva. Se introduce a propósito SIN `Default` para que el
//! compilador obligue a cada caller a decidir explícitamente el scope
//! durante el cutover.

/// Determina qué archivo de schema aplicar al inicializar una DB.
///
/// - `Local`: DB de proyecto (`.pillbox/pillbox.db`). Solo bottles,
///   prescriptions, pills y pills_fts.
/// - `Global`: DB de usuario (`~/.pillbox/pillbox.db`). Superset que
///   añade capsules, capsules_fts y registered_bottles.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DbScope {
    Local,
    Global,
}
