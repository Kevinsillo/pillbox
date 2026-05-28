//! Operaciones de store para la entidad [`Bottle`].

use anyhow::{Context, Result};
use rusqlite::{params, Connection, TransactionBehavior};
use uuid::Uuid;

use crate::{
    db::store::id_resolver::resolve_id,
    domain::{
        bottle::{Bottle, NewBottle},
        Paginated, PaginationParams,
    },
    error::PillboxError,
};

/// Crea un bottle nuevo en la DB.
pub fn create(conn: &mut Connection, input: &NewBottle) -> Result<Bottle> {
    let id = Uuid::now_v7().to_string();
    // BEGIN IMMEDIATE: adquiere el RESERVED lock al instante, evitando
    // "database is locked" cuando varios procesos/conexiones escriben en
    // paralelo. DEFERRED solo coge el lock al primer write y puede provocar
    // SQLITE_BUSY si dos transacciones lo intentan a la vez.
    let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;

    tx.execute(
        "INSERT INTO bottles (id, name, display_name, directory, scope)
         VALUES (?1, ?2, ?3, ?4, ?5)",
        params![
            id,
            input.name,
            input.display_name,
            input.directory,
            input.scope.as_str()
        ],
    )
    .map_err(|e| {
        if let rusqlite::Error::SqliteFailure(ref f, _) = e {
            if f.code == rusqlite::ErrorCode::ConstraintViolation {
                return anyhow::anyhow!(PillboxError::BottleAlreadyExists {
                    name: input.name.clone(),
                });
            }
        }
        anyhow::anyhow!(e).context("failed to create bottle")
    })?;

    let bottle = tx
        .query_row(
            "SELECT id, name, display_name, directory, scope, created_at, last_seen_at,
                views
         FROM bottles WHERE id = ?1",
            params![id],
            row_to_bottle,
        )
        .context("failed to read newly created bottle")?;

    tx.commit()?;
    Ok(bottle)
}

/// Lista todos los bottles ordenados por fecha de creación (paginado).
pub fn list(conn: &Connection, pagination: &PaginationParams) -> Result<Paginated<Bottle>> {
    let total: u64 = conn.query_row("SELECT COUNT(*) FROM bottles", [], |r| r.get(0))?;
    let limit = pagination.limit() as i64;
    let offset = pagination.offset() as i64;

    let mut stmt = conn.prepare(
        "SELECT id, name, display_name, directory, scope, created_at, last_seen_at,
                views
         FROM bottles ORDER BY created_at DESC
         LIMIT ?1 OFFSET ?2",
    )?;

    let items = stmt
        .query_map(params![limit, offset], row_to_bottle)?
        .collect::<rusqlite::Result<Vec<_>>>()
        .context("failed to list bottles")?;

    Ok(Paginated {
        items,
        total,
        page: pagination.page,
        page_size: pagination.page_size,
        used_fuzzy: false,
    })
}

/// Busca un bottle por su ID (UUID completo o prefijo ≥8 chars).
pub fn find_by_id(conn: &Connection, id: &str) -> Result<Option<Bottle>> {
    let Some(resolved_id) = resolve_id(conn, "bottles", id)? else {
        return Ok(None);
    };
    match conn.query_row(
        "SELECT id, name, display_name, directory, scope, created_at, last_seen_at,
                views
         FROM bottles WHERE id = ?1",
        params![resolved_id],
        row_to_bottle,
    ) {
        Ok(b) => Ok(Some(b)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(e).context("failed to find bottle by id"),
    }
}

/// Busca un bottle por el directorio del proyecto.
pub fn find_by_directory(conn: &Connection, directory: &str) -> Result<Option<Bottle>> {
    match conn.query_row(
        "SELECT id, name, display_name, directory, scope, created_at, last_seen_at,
                views
         FROM bottles WHERE directory = ?1",
        params![directory],
        row_to_bottle,
    ) {
        Ok(b) => Ok(Some(b)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(e).context("failed to find bottle by directory"),
    }
}

/// Elimina un bottle y en cascada sus prescriptions y pills.
///
/// Acepta UUID completo o prefijo ≥8 chars. El orden de borrado respeta las
/// FK: `pills` antes que `prescriptions`, y `prescriptions` antes que `bottles`.
///
/// Devuelve `Ok(true)` si el bottle existía y fue eliminado, `Ok(false)` si no existía.
pub fn delete(conn: &mut Connection, id: &str) -> Result<bool> {
    let Some(resolved_id) = resolve_id(conn, "bottles", id)? else {
        return Ok(false);
    };
    // BEGIN IMMEDIATE — ver nota en `create`. El delete encadena varios
    // DELETE en cascada manual; sin lock inmediato dos procesos podrían
    // colisionar a mitad de la cascada.
    let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;

    tx.execute(
        "DELETE FROM pills WHERE prescription_id IN (SELECT id FROM prescriptions WHERE bottle_id = ?1)",
        params![resolved_id],
    )
    .context("failed to delete bottle pills")?;

    tx.execute(
        "DELETE FROM prescriptions WHERE bottle_id = ?1",
        params![resolved_id],
    )
    .context("failed to delete bottle prescriptions")?;

    let count = tx
        .execute("DELETE FROM bottles WHERE id = ?1", params![resolved_id])
        .context("failed to delete bottle")?;

    tx.commit()?;
    Ok(count > 0)
}

/// Actualiza el `scope` de un bottle identificado por su `name`.
pub fn set_scope_by_name(conn: &Connection, name: &str, scope: &str) -> Result<()> {
    conn.execute(
        "UPDATE bottles SET scope = ?1 WHERE name = ?2",
        params![scope, name],
    )
    .context("failed to update bottle scope by name")?;
    Ok(())
}

/// Mapea una fila de SQLite al tipo [`Bottle`].
fn row_to_bottle(row: &rusqlite::Row<'_>) -> rusqlite::Result<Bottle> {
    Ok(Bottle {
        id: row.get(0)?,
        name: row.get(1)?,
        display_name: row.get(2)?,
        directory: row.get(3)?,
        scope: row.get(4)?,
        created_at: row.get(5)?,
        last_seen_at: row.get(6)?,
        linked: true,
        reg_id: None,
        views: row.get(7)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::connection::open_in_memory;
    use crate::db::DbScope;
    use crate::domain::bottle::{BottleScope, NewBottle};

    fn test_bottle(name: &str, dir: &str) -> NewBottle {
        NewBottle {
            name: name.into(),
            display_name: name.to_uppercase(),
            directory: dir.into(),
            scope: BottleScope::Local,
        }
    }

    #[test]
    fn create_and_find() {
        let mut conn = open_in_memory(DbScope::Local).unwrap();
        let b = create(&mut conn, &test_bottle("mi-proyecto", "/tmp/mi-proyecto")).unwrap();
        assert_eq!(b.name, "mi-proyecto");
        assert_eq!(b.scope, "local");

        let found = find_by_id(&conn, &b.id).unwrap().unwrap();
        assert_eq!(found.id, b.id);
    }

    #[test]
    fn uuid_is_generated_on_create() {
        let mut conn = open_in_memory(DbScope::Local).unwrap();
        let b = create(&mut conn, &test_bottle("uuid-test", "/tmp/uuid-test")).unwrap();
        assert!(!b.id.is_empty());
        assert_eq!(b.id.len(), 36);
        assert!(b.id.contains('-'));
    }

    #[test]
    fn two_bottles_have_different_ids() {
        let mut conn = open_in_memory(DbScope::Local).unwrap();
        let a = create(&mut conn, &test_bottle("a", "/tmp/a")).unwrap();
        let b = create(&mut conn, &test_bottle("b", "/tmp/b")).unwrap();
        assert_ne!(a.id, b.id);
    }

    #[test]
    fn find_by_directory_ok() {
        let mut conn = open_in_memory(DbScope::Local).unwrap();
        create(&mut conn, &test_bottle("proj", "/home/kevin/proj")).unwrap();
        let found = find_by_directory(&conn, "/home/kevin/proj").unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().name, "proj");
    }

    #[test]
    fn list_returns_all() {
        let mut conn = open_in_memory(DbScope::Local).unwrap();
        create(&mut conn, &test_bottle("a", "/tmp/a")).unwrap();
        create(&mut conn, &test_bottle("b", "/tmp/b")).unwrap();
        let page = list(&conn, &PaginationParams::default()).unwrap();
        assert_eq!(page.items.len(), 2);
        assert_eq!(page.total, 2);
    }

    #[test]
    fn duplicate_name_fails() {
        let mut conn = open_in_memory(DbScope::Local).unwrap();
        create(&mut conn, &test_bottle("dup", "/tmp/dup1")).unwrap();
        let err = create(&mut conn, &test_bottle("dup", "/tmp/dup2")).unwrap_err();
        let typed = err.downcast::<PillboxError>().unwrap();
        assert!(matches!(typed, PillboxError::BottleAlreadyExists { .. }));
    }

    #[test]
    fn find_by_id_missing_returns_none() {
        let conn = open_in_memory(DbScope::Local).unwrap();
        assert!(find_by_id(&conn, "id-inexistente").unwrap().is_none());
    }

    #[test]
    fn find_by_directory_missing_returns_none() {
        let conn = open_in_memory(DbScope::Local).unwrap();
        assert!(find_by_directory(&conn, "/ruta/inexistente")
            .unwrap()
            .is_none());
    }

    #[test]
    fn list_empty_returns_empty_vec() {
        let conn = open_in_memory(DbScope::Local).unwrap();
        let page = list(&conn, &PaginationParams::default()).unwrap();
        assert!(page.items.is_empty());
        assert_eq!(page.total, 0);
    }

    #[test]
    fn find_by_id_12char_prefix() {
        let mut conn = open_in_memory(DbScope::Local).unwrap();
        let b = create(&mut conn, &test_bottle("prefix-12", "/tmp/prefix-12")).unwrap();
        let short = b.id.replace('-', "").chars().take(12).collect::<String>();
        let found = find_by_id(&conn, &short).unwrap().unwrap();
        assert_eq!(found.id, b.id);
    }

    #[test]
    fn find_by_id_too_short_returns_invalid_id() {
        let conn = open_in_memory(DbScope::Local).unwrap();
        let err = find_by_id(&conn, "abc").unwrap_err();
        let typed = err.downcast_ref::<PillboxError>().unwrap();
        assert!(matches!(typed, PillboxError::InvalidId { .. }));
    }

    #[test]
    fn find_by_id_ambiguous_returns_ambiguous_id() {
        let conn = open_in_memory(DbScope::Local).unwrap();
        conn.execute(
            "INSERT INTO bottles (id, name, display_name, directory, scope)
             VALUES ('01234567-aaaa-7000-8000-000000000001', 'amb-a', 'A', '/tmp/amb-a', 'local')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO bottles (id, name, display_name, directory, scope)
             VALUES ('01234567-aaaa-7000-8000-000000000002', 'amb-b', 'B', '/tmp/amb-b', 'local')",
            [],
        )
        .unwrap();
        let err = find_by_id(&conn, "01234567aaaa").unwrap_err();
        let typed = err.downcast_ref::<PillboxError>().unwrap();
        assert!(matches!(typed, PillboxError::AmbiguousId { .. }));
    }

    #[test]
    fn list_paginates_correctly() {
        let mut conn = open_in_memory(DbScope::Local).unwrap();
        for i in 0..25 {
            create(
                &mut conn,
                &test_bottle(&format!("b{:02}", i), &format!("/tmp/b{:02}", i)),
            )
            .unwrap();
        }

        let p1 = list(
            &conn,
            &PaginationParams {
                page: 1,
                page_size: 20,
            },
        )
        .unwrap();
        assert_eq!(p1.items.len(), 20);
        assert_eq!(p1.total, 25);
        assert_eq!(p1.page, 1);

        let p2 = list(
            &conn,
            &PaginationParams {
                page: 2,
                page_size: 20,
            },
        )
        .unwrap();
        assert_eq!(p2.items.len(), 5);
        assert_eq!(p2.total, 25);
        assert_eq!(p2.page, 2);

        let p10 = list(
            &conn,
            &PaginationParams {
                page: 10,
                page_size: 20,
            },
        )
        .unwrap();
        assert_eq!(p10.items.len(), 0);
        assert_eq!(p10.total, 25);
        assert_eq!(p10.page, 10);
    }

    #[test]
    fn delete_by_short_id() {
        let mut conn = open_in_memory(DbScope::Local).unwrap();
        let b = create(&mut conn, &test_bottle("del-short", "/tmp/del-short")).unwrap();
        let short = b.id.replace('-', "").chars().take(12).collect::<String>();
        assert!(delete(&mut conn, &short).unwrap());
        assert!(find_by_id(&conn, &b.id).unwrap().is_none());
    }
}
