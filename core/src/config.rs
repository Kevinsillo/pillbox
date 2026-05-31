//! Constantes y rutas de configuración de Pillbox.
//!
//! Centraliza todas las rutas del sistema (`~/.pillbox/`, `.pillbox/`) y
//! los límites de validación usados en múltiples módulos.

use std::path::{Path, PathBuf};

/// Puerto por defecto del servidor HTTP.
pub const DEFAULT_PORT: u16 = 4242;

/// Límite máximo de caracteres para el contenido de una pill/capsule.
pub const CONTENT_MAX_LEN: usize = 5_000;

/// Límite máximo de caracteres para el título.
pub const TITLE_MAX_LEN: usize = 255;

/// Límite por defecto de registros archivados mostrados en listados del CLI.
pub const ARCHIVED_LIMIT_DEFAULT: u32 = 5;

/// Nombre del directorio local de la pillbox dentro de un proyecto.
pub const LOCAL_DIR: &str = ".pillbox";

/// Nombre del fichero de base de datos.
pub const DB_FILENAME: &str = "pillbox.db";

/// Resuelve la ruta de la DB según el `cwd` indicado.
///
/// Orden de prioridad:
/// 1. `{cwd}/.pillbox/pillbox.db` — DB local del proyecto (si existe)
/// 2. `~/.pillbox/pillbox.db` — DB global del usuario
///
/// Si ninguna existe, devuelve `None`. El caller debe devolver un error claro al usuario:
/// "DB global no encontrada. Reinstala con: curl -fsSL .../install.sh | bash"
/// No se crea ninguna DB silenciosamente fuera del instalador (`install.sh`).
///
/// El parámetro `cwd` se introduce para permitir a Fases 2/3 inyectar el
/// directorio de trabajo del cliente (axum request, MCP loop) en vez de
/// confiar en `std::env::current_dir()` del proceso servidor. En Fase 1 los
/// callers existentes usan el wrapper [`resolve_db_path_from_env`] que sigue
/// leyendo `current_dir()`.
pub fn resolve_db_path(cwd: &Path) -> Option<PathBuf> {
    let local = cwd.join(LOCAL_DIR).join(DB_FILENAME);
    let global = global_db_path();

    if local.exists() {
        // Si cwd == $HOME, la ruta local y la global apuntan al mismo archivo.
        // En ese caso, tratar como global para no confundir al caller.
        if local != global {
            return Some(local);
        }
    }

    if global.exists() {
        return Some(global);
    }

    None
}

/// Wrapper que invoca [`resolve_db_path`] usando `std::env::current_dir()`.
///
/// Compatibilidad para callers que aún no propagan `cwd` explícitamente
/// (subcomandos CLI one-shot, `pillbox serve` en Fase 1, `mcp::run` en Fase 1).
/// Fases 2 y 3 migrarán estos call-sites a [`resolve_db_path`] con el cwd
/// real del cliente — ver matriz en el design pill.
///
/// Callers actuales que dependen de este wrapper (Fase 1):
/// - `cmd::shared::open_db_with_path` — todos los subcomandos CLI one-shot
/// - `cmd::pill::cmd_pill_show`
/// - `cmd::status::run`
/// - `cmd::bottle::*` (vinculación)
/// - `cmd::serve::cmd_serve_run` (Fase 2 lo cambia por pool con cwd-driven)
/// - `mcp::execute` (Fase 3 lo cambia por loop con cwd en Request)
pub fn resolve_db_path_from_env() -> Option<PathBuf> {
    let cwd = std::env::current_dir().ok()?;
    resolve_db_path(&cwd)
}

/// Ruta de la DB local del proyecto: `./.pillbox/pillbox.db`
pub fn local_db_path() -> PathBuf {
    PathBuf::from(LOCAL_DIR).join(DB_FILENAME)
}

/// Ruta de la DB global del usuario: `~/.pillbox/pillbox.db`
pub fn global_db_path() -> PathBuf {
    dirs::home_dir()
        .expect("failed to resolve home directory")
        .join(format!(".{}", env!("CARGO_PKG_NAME")))
        .join(DB_FILENAME)
}

/// Ruta del servidor MCP: `~/.pillbox/mcp/index.js`
pub fn mcp_path() -> PathBuf {
    dirs::home_dir()
        .expect("failed to resolve home directory")
        .join(format!(".{}", env!("CARGO_PKG_NAME")))
        .join("mcp")
        .join("index.js")
}

/// Ruta de la skill de Claude Code: `~/.claude/skills/pillbox/SKILL.md`
pub fn skill_path() -> PathBuf {
    dirs::home_dir()
        .expect("failed to resolve home directory")
        .join(".claude")
        .join("skills")
        .join(env!("CARGO_PKG_NAME"))
        .join("SKILL.md")
}

/// Ruta del fichero de configuración de Claude Code: `~/.claude.json`
pub fn claude_config_path() -> PathBuf {
    dirs::home_dir()
        .expect("failed to resolve home directory")
        .join(".claude.json")
}

/// Ruta del fichero de configuración de OpenCode: `~/.config/opencode/opencode.json`
pub fn opencode_config_path() -> PathBuf {
    dirs::home_dir()
        .expect("failed to resolve home directory")
        .join(".config")
        .join("opencode")
        .join("opencode.json")
}

/// Ruta de la skill de OpenCode: `~/.config/opencode/skill/pillbox/SKILL.md`
///
/// OpenCode usa el directorio singular `skill`, a diferencia del `skills` de Claude Code.
pub fn opencode_skill_path() -> PathBuf {
    dirs::home_dir()
        .expect("failed to resolve home directory")
        .join(".config")
        .join("opencode")
        .join("skill")
        .join(env!("CARGO_PKG_NAME"))
        .join("SKILL.md")
}

/// Proveedor de asistente IA soportado por Pillbox.
///
/// Abstrae las diferencias de instalación entre Claude Code y OpenCode: ruta del
/// fichero de configuración MCP, ruta de la skill y clave del registro MCP. El resto
/// del código resuelve un `Provider` (vía flag `--provider`, detección o prompt) y
/// consulta estos métodos, manteniendo la ruta de Claude intacta por defecto.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Provider {
    Claude,
    OpenCode,
}

impl Provider {
    /// Todos los proveedores soportados, en orden de presentación.
    pub const ALL: [Provider; 2] = [Provider::Claude, Provider::OpenCode];

    /// Identificador estable usado por el flag `--provider` (minúsculas).
    pub fn id(self) -> &'static str {
        match self {
            Provider::Claude => "claude",
            Provider::OpenCode => "opencode",
        }
    }

    /// Nombre legible para mensajes y prompts.
    pub fn label(self) -> &'static str {
        match self {
            Provider::Claude => "Claude Code",
            Provider::OpenCode => "OpenCode",
        }
    }

    /// Parsea el valor del flag `--provider` (case-insensitive). `None` si no se reconoce.
    pub fn parse(value: &str) -> Option<Provider> {
        match value.to_lowercase().as_str() {
            "claude" | "claude-code" => Some(Provider::Claude),
            "opencode" => Some(Provider::OpenCode),
            _ => None,
        }
    }

    /// Directorio cuya existencia indica que el proveedor está instalado.
    ///
    /// Coincide con la detección de `skills/install.sh` (única fuente de verdad):
    /// `~/.claude` para Claude Code, `~/.config/opencode` para OpenCode.
    fn detect_dir(self) -> PathBuf {
        let home = dirs::home_dir().expect("failed to resolve home directory");
        match self {
            Provider::Claude => home.join(".claude"),
            Provider::OpenCode => home.join(".config").join("opencode"),
        }
    }

    /// `true` si el directorio de configuración del proveedor existe.
    pub fn is_installed(self) -> bool {
        self.detect_dir().is_dir()
    }

    /// Proveedores detectados como instalados, en el orden de [`Provider::ALL`].
    pub fn detect() -> Vec<Provider> {
        Self::ALL.into_iter().filter(|p| p.is_installed()).collect()
    }

    /// Ruta del fichero de configuración MCP del proveedor.
    pub fn mcp_config_path(self) -> PathBuf {
        match self {
            Provider::Claude => claude_config_path(),
            Provider::OpenCode => opencode_config_path(),
        }
    }

    /// Ruta del fichero `SKILL.md` instalado para el proveedor.
    pub fn skill_path(self) -> PathBuf {
        match self {
            Provider::Claude => skill_path(),
            Provider::OpenCode => opencode_skill_path(),
        }
    }

    /// Clave con punto bajo la que vive la entrada MCP de Pillbox en la config del proveedor.
    ///
    /// Claude Code anida bajo `mcpServers`; OpenCode bajo `mcp`.
    pub fn mcp_key(self) -> &'static str {
        match self {
            Provider::Claude => "mcpServers.pillbox",
            Provider::OpenCode => "mcp.pillbox",
        }
    }
}

/// Ruta donde el servicio de sistema persiste el puerto activo: `~/.pillbox/serve.port`
pub fn serve_port_path() -> PathBuf {
    dirs::home_dir()
        .expect("failed to resolve home directory")
        .join(format!(".{}", env!("CARGO_PKG_NAME")))
        .join("serve.port")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn global_db_path_contains_pillbox() {
        let path = global_db_path();
        assert!(path.to_string_lossy().contains("pillbox"));
        assert!(path.ends_with(DB_FILENAME));
    }

    #[test]
    fn local_db_path_is_relative() {
        let path = local_db_path();
        assert!(path.starts_with(LOCAL_DIR));
    }

    #[test]
    fn mcp_path_ends_with_index_js() {
        let path = mcp_path();
        assert!(path.ends_with("mcp/index.js"));
        assert!(path.to_string_lossy().contains("pillbox"));
    }

    #[test]
    fn skill_path_ends_with_skill_md() {
        let path = skill_path();
        assert!(path.ends_with("pillbox/SKILL.md"));
        assert!(path.to_string_lossy().contains(".claude"));
    }

    #[test]
    fn claude_config_path_ends_with_dot_claude_json() {
        let path = claude_config_path();
        assert!(path.ends_with(".claude.json"));
    }

    #[test]
    fn serve_port_path_ends_with_serve_port() {
        let path = serve_port_path();
        assert!(path.ends_with(".pillbox/serve.port"));
    }

    #[test]
    fn opencode_config_path_points_to_opencode_json() {
        let path = opencode_config_path();
        assert!(path.ends_with("opencode.json"));
        assert!(path.to_string_lossy().contains("opencode"));
    }

    #[test]
    fn opencode_skill_path_uses_singular_skill_dir() {
        let segments: Vec<String> = opencode_skill_path()
            .components()
            .map(|c| c.as_os_str().to_string_lossy().into_owned())
            .collect();
        assert!(segments.iter().any(|s| s == "opencode"));
        // OpenCode usa el directorio singular `skill`, nunca el plural `skills`.
        assert!(segments.iter().any(|s| s == "skill"));
        assert!(!segments.iter().any(|s| s == "skills"));
        assert!(segments.iter().any(|s| s == "pillbox"));
        assert!(opencode_skill_path().ends_with("SKILL.md"));
    }

    #[test]
    fn provider_parse_is_case_insensitive() {
        assert_eq!(Provider::parse("claude"), Some(Provider::Claude));
        assert_eq!(Provider::parse("Claude"), Some(Provider::Claude));
        assert_eq!(Provider::parse("claude-code"), Some(Provider::Claude));
        assert_eq!(Provider::parse("opencode"), Some(Provider::OpenCode));
        assert_eq!(Provider::parse("OpenCode"), Some(Provider::OpenCode));
        assert_eq!(Provider::parse("vscode"), None);
        assert_eq!(Provider::parse(""), None);
    }

    #[test]
    fn provider_paths_and_keys_per_variant() {
        assert_eq!(Provider::Claude.mcp_config_path(), claude_config_path());
        assert_eq!(Provider::OpenCode.mcp_config_path(), opencode_config_path());
        assert_eq!(Provider::Claude.skill_path(), skill_path());
        assert_eq!(Provider::OpenCode.skill_path(), opencode_skill_path());
        assert_eq!(Provider::Claude.mcp_key(), "mcpServers.pillbox");
        assert_eq!(Provider::OpenCode.mcp_key(), "mcp.pillbox");
        assert_eq!(Provider::Claude.id(), "claude");
        assert_eq!(Provider::OpenCode.id(), "opencode");
    }
}
