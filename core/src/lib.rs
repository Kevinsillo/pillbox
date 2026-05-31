//! Crate de biblioteca de Pillbox — lógica de negocio, acceso a DB y dominio.
//!
//! Expone los módulos `config`, `db`, `domain`, `error`, `normalize` y `server`
//! que usan tanto el binario `pillbox` como los tests de integración.

rust_i18n::i18n!("locales", fallback = "en");

pub mod config;
pub mod db;
pub mod domain;
pub mod error;
pub mod normalize;
pub mod server;
