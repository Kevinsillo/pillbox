//! Implementación de los subcomandos del CLI de Pillbox.
//!
//! Cada sub-módulo corresponde a un grupo de comandos (`pillbox bottle`,
//! `pillbox pill`, etc.). Las funciones públicas de cada módulo son invocadas
//! directamente desde `main`.

pub mod bottle;
pub mod capsule;
pub mod install;
pub mod lang;
pub mod mcp;
pub mod pill;
pub mod prescription;
pub mod serve;
pub mod shared;
pub mod skill;
pub mod status;
pub mod uninstall;
