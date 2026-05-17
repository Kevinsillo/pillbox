//! Constantes y rutas de configuración de Pillbox.
//!
//! Centraliza todas las rutas del sistema (`~/.pillbox/`, `.pillbox/`) y
//! los límites de validación usados en múltiples módulos.

use std::path::PathBuf;

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

/// Resuelve la ruta de la DB según el contexto de ejecución.
///
/// Orden de prioridad:
/// 1. `./.pillbox/pillbox.db` — DB local del proyecto (si existe)
/// 2. `~/.pillbox/pillbox.db` — DB global del usuario
///
/// Si ninguna existe, devuelve `None`. El caller debe devolver un error claro al usuario:
/// "DB global no encontrada. Reinstala con: curl -fsSL .../install.sh | bash"
/// No se crea ninguna DB silenciosamente fuera del instalador (`install.sh`).
pub fn resolve_db_path() -> Option<PathBuf> {
    let local = local_db_path();
    let global = global_db_path();

    if local.exists() {
        // Si cwd == $HOME, la ruta local y la global apuntan al mismo archivo.
        // En ese caso, tratar como global para no confundir al caller.
        let abs_local = std::env::current_dir().ok()?.join(&local);
        if abs_local != global {
            return Some(local);
        }
    }

    if global.exists() {
        return Some(global);
    }

    None
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
}
