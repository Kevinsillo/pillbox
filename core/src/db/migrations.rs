use anyhow::{Context, Result};
use rusqlite::Connection;

/// Última versión de schema conocida en tiempo de compilación.
pub const CURRENT_SCHEMA_VERSION: i64 = 1;

const MIGRATION_001: &str = include_str!("migrations/001_initial.sql");

/// Aplica todas las migraciones pendientes en orden.
///
/// Lee la versión actual desde `schema_migrations` (si existe) y aplica
/// solo las migraciones posteriores. Cada migración se ejecuta en una
/// transacción atómica — si falla, la DB queda en el estado anterior.
pub fn run(conn: &Connection) -> Result<()> {
    let current = current_version(conn)?;

    if current > CURRENT_SCHEMA_VERSION {
        anyhow::bail!(
            "DB schema v{} encontrado, v{} esperado. La versión de pillbox es más antigua que la DB.",
            current,
            CURRENT_SCHEMA_VERSION
        );
    }

    if current == CURRENT_SCHEMA_VERSION {
        return Ok(());
    }

    // Aplica migraciones desde current+1 hasta CURRENT_SCHEMA_VERSION
    if current < 1 {
        apply(conn, 1, MIGRATION_001).context("migración 001_initial falló")?;
    }

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
        .with_context(|| format!("error al aplicar migración v{}", version))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;

    #[test]
    fn migrations_apply_on_empty_db() {
        let conn = Connection::open_in_memory().unwrap();
        run(&conn).unwrap();

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
        run(&conn).unwrap();
        run(&conn).unwrap(); // segunda vez no debe fallar
    }

    #[test]
    fn pill_compounds_seeded() {
        let conn = Connection::open_in_memory().unwrap();
        run(&conn).unwrap();

        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM pill_compounds", [], |r| r.get(0))
            .unwrap();

        assert!(count > 0);
    }
}
