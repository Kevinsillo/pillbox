//! Capa de acceso a datos — conexiones SQLite, migraciones y operaciones de store.

pub mod connection;
pub mod migrate;
pub mod migrations;
pub mod schema;
pub mod scope;
pub mod store;

pub use scope::DbScope;
