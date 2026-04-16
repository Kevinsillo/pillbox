use owo_colors::OwoColorize;
use tabled::{builder::Builder, settings::Style};

/// Tabla con cabecera y filas de datos. Estilo rounded, sin separadores entre filas.
pub fn list(headers: &[&str], rows: Vec<Vec<String>>) -> String {
    let mut b = Builder::default();
    b.push_record(headers.iter().map(|h| h.bold().to_string()));
    for row in rows {
        b.push_record(row);
    }
    b.build().with(Style::rounded()).to_string()
}

/// Tabla de dos columnas clave/valor. Estilo modern_rounded, con separadores entre filas.
pub fn dict(rows: Vec<[String; 2]>) -> String {
    let mut b = Builder::default();
    for row in rows {
        b.push_record(row);
    }
    b.build().with(Style::modern_rounded()).to_string()
}
