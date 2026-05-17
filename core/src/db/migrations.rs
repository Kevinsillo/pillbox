use anyhow::{Context, Result};
use rusqlite::Connection;

use crate::db::DbScope;

/// Última versión de schema conocida en tiempo de compilación.
///
/// Hay un único contador de versión compartido entre los dos schemas (local y
/// global). El runner elige el SQL a aplicar según `DbScope`, no según número
/// de archivo: la versión 0 → 1 se materializa con uno u otro fichero.
pub const CURRENT_SCHEMA_VERSION: i64 = 1;

const SCHEMA_LOCAL: &str = include_str!("migrations/00_schema_local.sql");
const SCHEMA_GLOBAL: &str = include_str!("migrations/00_schema_global.sql");

/// Aplica el schema inicial correspondiente al `scope` si la DB está vacía.
///
/// Si la DB ya está en `CURRENT_SCHEMA_VERSION` no hace nada (idempotente).
/// Si la versión persistida es mayor que la conocida por el binario, falla
/// (binario más viejo que la DB).
///
/// Nota: en esta fase no hay migraciones incrementales — el salto v0 → v1 se
/// resuelve aplicando un único archivo elegido por scope.
pub fn run(conn: &Connection, scope: DbScope) -> Result<()> {
    let current = current_version(conn)?;

    if current > CURRENT_SCHEMA_VERSION {
        anyhow::bail!(
            "DB schema v{} found, v{} expected. The pillbox binary is older than the DB.",
            current,
            CURRENT_SCHEMA_VERSION
        );
    }

    if current == CURRENT_SCHEMA_VERSION {
        return Ok(());
    }

    // v0 → v1: aplicar el schema inicial según scope.
    let (sql, label) = match scope {
        DbScope::Local => (SCHEMA_LOCAL, "00_schema_local"),
        DbScope::Global => (SCHEMA_GLOBAL, "00_schema_global"),
    };

    apply(conn, 1, sql).with_context(|| format!("migration {} failed", label))?;

    Ok(())
}

/// Lee la versión actual del schema. Devuelve 0 si la tabla no existe todavía.
fn current_version(conn: &Connection) -> Result<i64> {
    let exists: bool = conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name='schema_migrations')",
        [],
        |row| row.get(0),
    )?;

    if !exists {
        return Ok(0);
    }

    let version: i64 = conn.query_row(
        "SELECT COALESCE(MAX(version), 0) FROM schema_migrations",
        [],
        |row| row.get(0),
    )?;

    Ok(version)
}

/// Ejecuta un bloque SQL de migración dentro de una transacción explícita.
/// Los PRAGMAs del archivo se omiten aquí — ya se aplicaron en `configure()`.
fn apply(conn: &Connection, version: i64, sql: &str) -> Result<()> {
    // Filtra líneas de PRAGMA — no son transaccionables y ya se aplican en configure()
    let ddl: String = sql
        .lines()
        .filter(|l| !l.trim_start().to_uppercase().starts_with("PRAGMA"))
        .collect::<Vec<_>>()
        .join("\n");

    conn.execute_batch(&format!("BEGIN;\n{}\nCOMMIT;", ddl))
        .with_context(|| format!("failed to apply migration v{}", version))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;

    #[test]
    fn migrations_apply_on_empty_db_local() {
        let conn = Connection::open_in_memory().unwrap();
        run(&conn, DbScope::Local).unwrap();

        let version: i64 = conn
            .query_row("SELECT MAX(version) FROM schema_migrations", [], |r| {
                r.get(0)
            })
            .unwrap();

        assert_eq!(version, CURRENT_SCHEMA_VERSION);
    }

    #[test]
    fn migrations_apply_on_empty_db_global() {
        let conn = Connection::open_in_memory().unwrap();
        run(&conn, DbScope::Global).unwrap();

        let version: i64 = conn
            .query_row("SELECT MAX(version) FROM schema_migrations", [], |r| {
                r.get(0)
            })
            .unwrap();

        assert_eq!(version, CURRENT_SCHEMA_VERSION);
    }

    #[test]
    fn migrations_idempotent() {
        let conn = Connection::open_in_memory().unwrap();
        run(&conn, DbScope::Local).unwrap();
        run(&conn, DbScope::Local).unwrap(); // segunda vez no debe fallar
    }

    #[test]
    fn schema_migrations_seeded() {
        let conn = Connection::open_in_memory().unwrap();
        run(&conn, DbScope::Global).unwrap();

        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM schema_migrations", [], |r| r.get(0))
            .unwrap();

        assert_eq!(count, 1);
    }
}
