pub const SUPPORTED: &[(&str, &str)] = &[
    ("es", "Español"),
    ("en", "English"),
    ("de", "Deutsch"),
    ("it", "Italiano"),
    ("pt", "Português"),
    ("fr", "Français"),
];

pub fn is_supported(code: &str) -> bool {
    SUPPORTED.iter().any(|(c, _)| *c == code)
}

pub fn lang_file_path() -> std::path::PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("/tmp"))
        .join(".pillbox")
        .join("lang")
}

pub fn detect() -> String {
    // 1. ~/.pillbox/lang
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
    // 3. System locale — native on Windows (Win32), macOS (CFLocale), Linux (env vars)
    if let Some(locale) = sys_locale::get_locale() {
        let code = locale
            .split(['-', '_', '.', '@'])
            .next()
            .unwrap_or("")
            .to_lowercase();
        if is_supported(&code) {
            return code;
        }
    }
    "es".to_string()
}

pub fn save(code: &str) -> std::io::Result<()> {
    let path = lang_file_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, format!("{}\n", code))
}
