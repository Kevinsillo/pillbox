use anyhow::Result;
use rust_i18n::t;

use crate::output;

pub fn run() -> Result<()> {
    use output::fmt::{StatusBottle, StatusDb};

    let bin_path = std::env::current_exe()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|_| t!("status.bin_unknown").to_string());

    let query_db = |path: &std::path::Path| -> Option<Result<(i64, i64, i64, i64, i64), String>> {
        if !path.exists() {
            return None;
        }
        Some((|| -> Result<_, String> {
            let conn = pillbox::db::connection::open(path).map_err(|e| e.to_string())?;
            conn.query_row(
                "SELECT
                         (SELECT MAX(version) FROM schema_migrations),
                         (SELECT COUNT(*) FROM bottles),
                         (SELECT COUNT(*) FROM pills         WHERE deleted_at IS NULL),
                         (SELECT COUNT(*) FROM capsules      WHERE deleted_at IS NULL),
                         (SELECT COUNT(*) FROM prescriptions WHERE deleted_at IS NULL)",
                [],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?)),
            )
            .map_err(|e| e.to_string())
        })())
    };

    let global_path = pillbox::config::global_db_path();
    let local_path = pillbox::config::local_db_path();
    let _ = pillbox::db::connection::open(&global_path);

    let global = StatusDb {
        result: query_db(&global_path),
        path: global_path.display().to_string(),
    };
    let local = StatusDb {
        result: query_db(&local_path),
        path: local_path.display().to_string(),
    };

    let bottle = pillbox::config::resolve_db_path()
        .and_then(|db_path| pillbox::db::connection::open(&db_path).ok())
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
