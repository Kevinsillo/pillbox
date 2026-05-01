//! Crate de biblioteca de Pillbox — lógica de negocio, acceso a DB y dominio.
//!
//! Expone los módulos `config`, `db`, `domain`, `error` y `normalize`
//! que usan tanto el binario `pillbox` como los tests de integración.

pub mod config;
pub mod db;
pub mod domain;
pub mod error;
pub mod normalize;
