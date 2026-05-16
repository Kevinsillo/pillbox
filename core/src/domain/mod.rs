//! Tipos de dominio de Pillbox — structs de entrada/salida y enumeraciones.
//!
//! Estos tipos no contienen lógica de negocio; son DTOs que fluyen entre
//! la capa de CLI/MCP/HTTP y la capa de persistencia (`db::store`).

pub mod bottle;
pub mod capsule;
pub mod pagination;
pub mod pill;
pub mod prescription;
pub mod search;

pub use pagination::{Paginated, PaginationParams};
