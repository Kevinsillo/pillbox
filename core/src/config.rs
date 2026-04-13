use std::path::{Path, PathBuf};

/// Puerto por defecto del servidor HTTP.
pub const DEFAULT_PORT: u16 = 4242;

/// Límite máximo de caracteres para el contenido de una pill/capsule.
pub const CONTENT_MAX_LEN: usize = 5_000;

/// Límite máximo de caracteres para el título.
pub const TITLE_MAX_LEN: usize = 255;

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
    if local.exists() {
        return Some(local);
    }

    let global = global_db_path();
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
        .expect("no se pudo resolver el directorio home")
        .join(format!(".{}", env!("CARGO_PKG_NAME")))
        .join(DB_FILENAME)
}

/// Indica si una ruta de DB corresponde a una pillbox local de proyecto.
pub fn is_local_db(path: &Path) -> bool {
    path.starts_with(LOCAL_DIR) || path.components().any(|c| c.as_os_str() == LOCAL_DIR)
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
}
