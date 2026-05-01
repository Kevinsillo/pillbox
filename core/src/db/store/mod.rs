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
