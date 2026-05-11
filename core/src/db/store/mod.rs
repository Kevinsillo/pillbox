//! Operaciones CRUD sobre las entidades de Pillbox.
//!
//! Cada sub-módulo agrupa las operaciones de una entidad: `bottles`, `pills`,
//! `capsules`, `prescriptions`, `registered_bottles` y `search` (FTS5 + fuzzy).

pub mod bottles;
pub mod capsules;
pub mod pills;
pub mod prescriptions;
pub mod registered_bottles;
pub mod search;

/// Filtro de estado para listar entidades con soft-delete.
///
/// Permite a los callers decidir si quieren registros activos, archivados o todos.
/// Usado en `prescriptions::list_by_bottle` y `capsules::list`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ListFilter {
    /// Solo registros activos (`deleted_at IS NULL`).
    Active,
    /// Solo registros archivados (`deleted_at IS NOT NULL`).
    Archived,
    /// Todos los registros (sin filtro de soft-delete).
    All,
}
