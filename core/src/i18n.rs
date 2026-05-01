//! Detección y gestión del idioma del CLI.
//!
//! El idioma se resuelve con la siguiente prioridad:
//! 1. `~/.pillbox/lang` — preferencia guardada por el usuario
//! 2. `PILLBOX_LANG` — variable de entorno de sobreescritura
//! 3. Locale del sistema (via `sys_locale`) — se guarda para futuras ejecuciones
//! 4. `"en"` — fallback final

/// Idiomas soportados con su nombre legible para el usuario.
pub const SUPPORTED: &[(&str, &str)] = &[
    ("es", "Español"),
    ("en", "English"),
    ("de", "Deutsch"),
    ("it", "Italiano"),
    ("pt", "Português"),
    ("fr", "Français"),
];

/// Devuelve `true` si `code` es un código de idioma soportado.
pub fn is_supported(code: &str) -> bool {
    SUPPORTED.iter().any(|(c, _)| *c == code)
}

/// Ruta del fichero de preferencia de idioma: `~/.pillbox/lang`
pub fn lang_file_path() -> std::path::PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("/tmp"))
        .join(".pillbox")
        .join("lang")
}

/// Detecta el idioma activo siguiendo el orden de prioridad del módulo.
///
/// En el primer arranque, detecta el locale del sistema y lo persiste en
/// `~/.pillbox/lang` para que futuras ejecuciones sean instantáneas.
pub fn detect() -> String {
    // 1. ~/.pillbox/lang — persisted preference
    if let Ok(content) = std::fs::read_to_string(lang_file_path()) {
        let code = content.trim().to_lowercase();
        if is_supported(&code) {
            return code;
        }
    }
    // 2. PILLBOX_LANG env var override
    if let Ok(val) = std::env::var("PILLBOX_LANG") {
        let code = val.trim().to_lowercase();
        if is_supported(&code) {
            return code;
        }
    }
    // 3. First run: detect system locale, save it and use it
    let detected = sys_locale::get_locale()
        .and_then(|locale| {
            let code = locale
                .split(['-', '_', '.', '@'])
                .next()
                .unwrap_or("")
                .to_lowercase();
            if is_supported(&code) {
                Some(code)
            } else {
                None
            }
        })
        .unwrap_or_else(|| "en".to_string());
    let _ = save(&detected);
    detected
}

/// Persiste el código de idioma en `~/.pillbox/lang`.
///
/// Crea el directorio padre si no existe.
pub fn save(code: &str) -> std::io::Result<()> {
    let path = lang_file_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, format!("{}\n", code))
}
