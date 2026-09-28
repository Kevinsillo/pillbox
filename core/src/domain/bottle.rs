//! Tipos de dominio para bottles — contenedores de conocimiento por proyecto.

use serde::{Deserialize, Serialize};
use validator::Validate;

/// Input para crear un bottle (desde `pillbox bottle init`).
#[derive(Debug, Serialize, Deserialize, Validate)]
pub struct NewBottle {
    /// Slug generado automáticamente del nombre de carpeta. No lo elige el usuario.
    #[validate(length(min = 1, max = 255))]
    pub name: String,

    /// Nombre legible elegido por el usuario en el wizard de init.
    #[validate(length(min = 1, max = 255))]
    pub display_name: String,

    /// Ruta absoluta al directorio del proyecto.
    #[validate(length(min = 1))]
    pub directory: String,

    /// Scope de la DB: 'local' (.pillbox/pillbox.db) o 'global' (~/.pillbox/pillbox.db).
    pub scope: BottleScope,
}

/// Scope de la DB donde vive el bottle.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BottleScope {
    /// DB local del proyecto: `.pillbox/pillbox.db`
    Local,
    /// DB global del usuario: `~/.pillbox/pillbox.db`
    Global,
}

impl BottleScope {
    /// Representación canónica en string para almacenar en DB.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Local => "local",
            Self::Global => "global",
        }
    }
}

impl std::fmt::Display for BottleScope {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Valor por defecto de `linked` al deserializar — `true` cuando el campo está ausente.
fn default_true() -> bool {
    true
}

/// Bottle tal como se devuelve al listar.
#[derive(Debug, Serialize, Deserialize)]
pub struct Bottle {
    pub id: String,
    pub name: String,
    pub display_name: String,
    pub directory: String,
    pub scope: String,
    pub created_at: String,
    pub last_seen_at: String,
    /// false cuando el bottle está registrado globalmente pero su DB ya no existe en disco.
    #[serde(default = "default_true")]
    pub linked: bool,
    /// ID del registro en `registered_bottles` (presente en bottles con DB registrada).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reg_id: Option<i64>,
    /// Contador de vistas. `#[serde(default)]` por compatibilidad con clientes API que no envían el campo.
    #[serde(default)]
    pub views: i64,
    /// Última actividad: `MAX(updated_at)` de las pills no borradas del bottle,
    /// con fallback a `created_at`. Formato SQLite `YYYY-MM-DD HH:MM:SS` (UTC).
    /// `None` cuando la DB registrada no es accesible. Se serializa siempre
    /// (como `null` si falta) para que el payload HTTP contenga la clave.
    #[serde(default)]
    pub last_activity_at: Option<String>,
}

/// Orden canónico de listados de bottles: última actividad DESC (los `None`
/// al final) y `display_name` ASC como desempate.
///
/// Opera sobre claves y no sobre [`Bottle`] porque el CLI ordena filas propias
/// (`BottleListRow`). La comparación textual de timestamps es válida porque
/// todas las escrituras usan `datetime('now')` (formato fijo, UTC).
pub fn cmp_by_activity(
    a_activity: Option<&str>,
    a_display_name: &str,
    b_activity: Option<&str>,
    b_display_name: &str,
) -> std::cmp::Ordering {
    use std::cmp::Ordering;
    let by_activity = match (a_activity, b_activity) {
        (Some(a), Some(b)) => b.cmp(a),
        (Some(_), None) => Ordering::Less,
        (None, Some(_)) => Ordering::Greater,
        (None, None) => Ordering::Equal,
    };
    by_activity.then_with(|| a_display_name.cmp(b_display_name))
}

/// Ordena bottles in-place según [`cmp_by_activity`].
pub fn sort_by_activity(bottles: &mut [Bottle]) {
    bottles.sort_by(|a, b| {
        cmp_by_activity(
            a.last_activity_at.as_deref(),
            &a.display_name,
            b.last_activity_at.as_deref(),
            &b.display_name,
        )
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cmp::Ordering;

    #[test]
    fn cmp_by_activity_most_recent_first() {
        assert_eq!(
            cmp_by_activity(
                Some("2026-09-02 10:00:00"),
                "a",
                Some("2026-09-01 10:00:00"),
                "b"
            ),
            Ordering::Less
        );
        assert_eq!(
            cmp_by_activity(
                Some("2026-09-01 10:00:00"),
                "a",
                Some("2026-09-02 10:00:00"),
                "b"
            ),
            Ordering::Greater
        );
    }

    #[test]
    fn cmp_by_activity_none_goes_last() {
        assert_eq!(
            cmp_by_activity(None, "a", Some("2000-01-01 00:00:00"), "z"),
            Ordering::Greater
        );
        assert_eq!(
            cmp_by_activity(Some("2000-01-01 00:00:00"), "z", None, "a"),
            Ordering::Less
        );
    }

    #[test]
    fn cmp_by_activity_ties_break_by_display_name() {
        let ts = Some("2026-09-01 10:00:00");
        assert_eq!(cmp_by_activity(ts, "Alpha", ts, "Beta"), Ordering::Less);
        assert_eq!(
            cmp_by_activity(None, "Beta", None, "Alpha"),
            Ordering::Greater
        );
    }

    #[test]
    fn sort_by_activity_orders_slice() {
        let mk = |name: &str, act: Option<&str>| Bottle {
            id: String::new(),
            name: name.to_lowercase(),
            display_name: name.into(),
            directory: String::new(),
            scope: "local".into(),
            created_at: String::new(),
            last_seen_at: String::new(),
            linked: true,
            reg_id: None,
            views: 0,
            last_activity_at: act.map(str::to_string),
        };
        let mut v = vec![
            mk("Zeta", None),
            mk("Beta", Some("2026-09-01 10:00:00")),
            mk("Alpha", Some("2026-09-01 10:00:00")),
            mk("Gamma", Some("2026-09-03 10:00:00")),
        ];
        sort_by_activity(&mut v);
        let names: Vec<_> = v.iter().map(|b| b.display_name.as_str()).collect();
        assert_eq!(names, ["Gamma", "Alpha", "Beta", "Zeta"]);
    }
}
