//! Utilidades de presentación en terminal: formato, tablas y truncado de texto.

pub(crate) mod bottle;
pub(crate) mod capsule;
pub(crate) mod flow;
pub(crate) mod layout;
pub(crate) mod logo;
pub(crate) mod mcp;
pub(crate) mod pill;
pub(crate) mod prescription;
pub(crate) mod status;
pub(crate) mod table;

/// Trunca una cadena Unicode a un máximo de caracteres, añadiendo `…` si se recorta.
pub(crate) fn truncate(s: &str, max: usize) -> String {
    let chars: Vec<char> = s.chars().collect();
    if chars.len() <= max {
        s.to_string()
    } else {
        format!(
            "{}…",
            chars[..max.saturating_sub(1)].iter().collect::<String>()
        )
    }
}

/// Backward-compatible façade re-exporting every public fn/type that previously
/// lived in `output::fmt`. New code should import from the responsibility
/// modules directly (`output::bottle`, `output::status`, ...).
pub mod fmt {
    pub use super::bottle::{
        bottle_init_created, bottle_init_done, bottle_init_gitignore, bottle_init_start,
        bottle_status, bottle_vinculate_already, bottle_vinculate_done, bottles_registered_list,
        BottleListRow,
    };
    pub use super::capsule::{capsule_detail, capsules_list};
    pub use super::flow::{
        migrate_confirm_global, migrate_confirm_local, migrate_help, migrate_result_global,
        migrate_result_local,
    };
    pub use super::logo::print_logo;
    pub use super::mcp::{
        mcp_not_installed, mcp_uninstalled, skill_not_installed, skill_uninstalled,
    };
    pub use super::pill::pill_detail;
    pub use super::prescription::{
        prescription_closed, prescription_opened, prescription_reopened, prescription_show,
        prescriptions_list,
    };
    pub use super::status::{
        component_status_with_help, db_not_found, help_with_status, serve_started, serve_status,
        serve_stopped, status, StatusBottle, StatusDb, StatusDbResult,
    };
}
