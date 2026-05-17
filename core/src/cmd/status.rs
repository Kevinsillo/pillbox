//! Subcomando `pillbox status`.

use anyhow::Result;
use rust_i18n::t;

use crate::output;

/// Muestra el estado global del sistema: DBs, bottle activo, servidor, MCP y skill.
pub fn run() -> Result<()> {
    use output::fmt::{StatusBottle, StatusDb, StatusDbResult};
    use pillbox::db::DbScope;

    let bin_path = std::env::current_exe()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|_| t!("status.bin_unknown").to_string());

    let global_path = pillbox::config::global_db_path();
    let local_path = pillbox::config::local_db_path();

    let query_db = |path: &std::path::Path, scope: DbScope| -> StatusDbResult {
        if !path.exists() {
            return None;
        }
        Some((|| -> Result<_, String> {
            let conn = pillbox::db::connection::open(path, scope).map_err(|e| e.to_string())?;
            // Local schema no tiene `capsules`; el global no tiene esa diferencia.
            let capsules_count: i64 = if scope == DbScope::Global {
                conn.query_row(
                    "SELECT COUNT(*) FROM capsules WHERE deleted_at IS NULL",
                    [],
                    |r| r.get(0),
                )
                .map_err(|e| e.to_string())?
            } else {
                0
            };
            let (ver, b, p, rx): (i64, i64, i64, i64) = conn
                .query_row(
                    "SELECT
                         (SELECT MAX(version) FROM schema_migrations),
                         (SELECT COUNT(*) FROM bottles),
                         (SELECT COUNT(*) FROM pills         WHERE deleted_at IS NULL),
                         (SELECT COUNT(*) FROM prescriptions WHERE deleted_at IS NULL)",
                    [],
                    |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
                )
                .map_err(|e| e.to_string())?;
            Ok((ver, b, p, capsules_count, rx))
        })())
    };

    let _ = pillbox::db::connection::open(&global_path, DbScope::Global);

    let global = StatusDb {
        result: query_db(&global_path, DbScope::Global),
        path: global_path.display().to_string(),
    };
    let local = StatusDb {
        result: query_db(&local_path, DbScope::Local),
        path: local_path.display().to_string(),
    };

    let bottle = pillbox::config::resolve_db_path()
        .and_then(|db_path| {
            let scope = if db_path == global_path {
                DbScope::Global
            } else {
                DbScope::Local
            };
            pillbox::db::connection::open(&db_path, scope).ok()
        })
        .and_then(|conn| {
            let dir = std::env::current_dir().ok()?.to_string_lossy().to_string();
            pillbox::db::store::bottles::find_by_directory(&conn, &dir)
                .ok()
                .flatten()
                .map(|b| {
                    let open_rx = conn
                        .query_row(
                            "SELECT title FROM prescriptions
                             WHERE bottle_id = ?1 AND ended_at IS NULL AND deleted_at IS NULL
                             LIMIT 1",
                            rusqlite::params![b.id],
                            |r| r.get(0),
                        )
                        .ok();
                    StatusBottle { open_rx }
                })
        });

    let server_port = {
        use std::net::TcpStream;
        use std::time::Duration;
        let port = pillbox::config::DEFAULT_PORT;
        TcpStream::connect_timeout(
            &std::net::SocketAddr::from(([127, 0, 0, 1], port)),
            Duration::from_millis(150),
        )
        .ok()
        .map(|_| port)
    };

    output::fmt::status(
        &bin_path,
        global,
        local,
        bottle,
        server_port,
        &pillbox::config::mcp_path(),
        &pillbox::config::skill_path(),
    );
    Ok(())
}
