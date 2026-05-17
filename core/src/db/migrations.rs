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
/// Concurrencia (multi-proceso):
/// El check-then-apply se hace dentro de una transacción `BEGIN IMMEDIATE`
/// para serializar la inicialización: solo un proceso adquiere el lock y
/// ejecuta la DDL; el resto reintentan (gobernados por `busy_timeout`),
/// vuelven a leer `current_version` y ven el schema ya aplicado, así que
/// salen sin hacer nada. La DDL usa `IF NOT EXISTS` por defensa adicional
/// frente a cualquier ventana de carrera residual.
///
/// Nota: en esta fase no hay migraciones incrementales — el salto v0 → v1 se
/// resuelve aplicando un único archivo elegido por scope.
pub fn run(conn: &Connection, scope: DbScope) -> Result<()> {
    // Fast path: si ya está al día, evitamos abrir transacción de escritura
    // (útil para callers concurrentes — solo el primero coge el lock).
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

    // Slow path: tomar lock de escritura inmediato y volver a comprobar.
    // BEGIN IMMEDIATE adquiere el RESERVED lock al instante (no espera al
    // primer write como BEGIN DEFERRED), evitando "database is locked"
    // cuando varios procesos arrancan en paralelo.
    conn.execute_batch("BEGIN IMMEDIATE;")
        .context("failed to BEGIN IMMEDIATE for migration")?;

    // Re-leer versión bajo el lock — puede que otro proceso acabara
    // de aplicar la migración mientras esperábamos.
    let current = match current_version(conn) {
        Ok(v) => v,
        Err(e) => {
            let _ = conn.execute_batch("ROLLBACK;");
            return Err(e);
        }
    };

    if current >= CURRENT_SCHEMA_VERSION {
        conn.execute_batch("COMMIT;")
            .context("failed to COMMIT no-op migration tx")?;
        return Ok(());
    }

    let (sql, label) = match scope {
        DbScope::Local => (SCHEMA_LOCAL, "00_schema_local"),
        DbScope::Global => (SCHEMA_GLOBAL, "00_schema_global"),
    };

    // Filtra PRAGMA (no son transaccionables; ya se aplican en configure()).
    let ddl: String = sql
        .lines()
        .filter(|l| !l.trim_start().to_uppercase().starts_with("PRAGMA"))
        .collect::<Vec<_>>()
        .join("\n");

    if let Err(e) = conn
        .execute_batch(&ddl)
        .with_context(|| format!("failed to apply migration {} (v1)", label))
    {
        let _ = conn.execute_batch("ROLLBACK;");
        return Err(e);
    }

    // Marca la migración como aplicada. INSERT OR IGNORE por si la DDL
    // ya contiene el seed (los schemas actuales lo hacen) — evita duplicate
    // PK si el archivo seedea schema_migrations directamente.
    if let Err(e) = conn
        .execute(
            "INSERT OR IGNORE INTO schema_migrations (version, name) VALUES (?1, ?2)",
            rusqlite::params![CURRENT_SCHEMA_VERSION, label],
        )
        .context("failed to record schema_migrations row")
    {
        let _ = conn.execute_batch("ROLLBACK;");
        return Err(e);
    }

    conn.execute_batch("COMMIT;")
        .with_context(|| format!("failed to COMMIT migration {} (v1)", label))?;

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
