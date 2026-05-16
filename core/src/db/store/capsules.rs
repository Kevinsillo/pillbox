//! Operaciones de store para la entidad [`Capsule`].

use anyhow::{Context, Result};
use rusqlite::{params, Connection, TransactionBehavior};
use serde::Serialize;
use uuid::Uuid;

use crate::db::store::id_resolver::resolve_id;
use crate::db::store::ListFilter;
use crate::domain::capsule::{Capsule, CapsulePatch, NewCapsule};
use crate::domain::{Paginated, PaginationParams};

// ─── Tipos de resultado ───────────────────────────────────────────────────────

/// Resultado de guardar una capsule nueva.
#[derive(Debug, Serialize)]
pub struct CapsuleStoreResult {
    pub id: String,
    pub action: &'static str, // "created"
    pub title: String,
    pub compound: String,
    pub content: String,
}

/// Resultado de descartar una capsule (soft delete).
#[derive(Debug, Serialize)]
pub struct CapsuleDiscardResult {
    pub id: String,
    pub deleted_at: String,
}

// ─── Store ────────────────────────────────────────────────────────────────────

/// Guarda una capsule de conocimiento personal (sin prescription ni bottle).
pub fn take(conn: &mut Connection, input: &NewCapsule) -> Result<CapsuleStoreResult> {
    let id = Uuid::now_v7().to_string();
    let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;

    tx.execute(
        "INSERT INTO capsules (id, compound, title, content)
         VALUES (?1, ?2, ?3, ?4)",
        params![id, input.compound.as_str(), input.title, input.content],
    )
    .context("failed to insert capsule")?;

    tx.commit()?;
    Ok(CapsuleStoreResult {
        id,
        action: "created",
        title: input.title.clone(),
        compound: input.compound.as_str().to_string(),
        content: input.content.clone(),
    })
}

/// Lee una capsule completa por UUID completo o prefijo ≥8 chars (no descartada).
pub fn read(conn: &Connection, id: &str) -> Result<Option<Capsule>> {
    let Some(resolved_id) = resolve_id(conn, "capsules", id)? else {
        return Ok(None);
    };
    match conn.query_row(
        "SELECT id, compound, title, content, created_at, updated_at, deleted_at
         FROM capsules WHERE id = ?1 AND deleted_at IS NULL",
        params![resolved_id],
        row_to_capsule,
    ) {
        Ok(c) => Ok(Some(c)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(e).context("failed to read capsule"),
    }
}

/// Lee una capsule completa por UUID completo o prefijo ≥8 chars, incluyendo descartadas.
pub fn read_any(conn: &Connection, id: &str) -> Result<Option<Capsule>> {
    let Some(resolved_id) = resolve_id(conn, "capsules", id)? else {
        return Ok(None);
    };
    match conn.query_row(
        "SELECT id, compound, title, content, created_at, updated_at, deleted_at
         FROM capsules WHERE id = ?1",
        params![resolved_id],
        row_to_capsule,
    ) {
        Ok(c) => Ok(Some(c)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(e).context("failed to read capsule"),
    }
}

/// Actualiza campos de una capsule existente (patch parcial).
///
/// Acepta UUID completo o prefijo ≥8 chars. Devuelve `None` si no existe
/// o fue descartada.
pub fn revise(conn: &mut Connection, id: &str, patch: &CapsulePatch) -> Result<Option<Capsule>> {
    let Some(resolved_id) = resolve_id(conn, "capsules", id)? else {
        return Ok(None);
    };

    let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;

    let affected = tx
        .execute(
            "UPDATE capsules
         SET title    = COALESCE(?1, title),
             content  = COALESCE(?2, content),
             compound = COALESCE(?3, compound),
             updated_at = datetime('now')
         WHERE id = ?4 AND deleted_at IS NULL",
            params![
                patch.title.as_deref(),
                patch.content.as_deref(),
                patch.compound.as_ref().map(|c| c.as_str()),
                resolved_id,
            ],
        )
        .context("failed to update capsule")?;

    if affected == 0 {
        return Ok(None);
    }

    let capsule = tx
        .query_row(
            "SELECT id, compound, title, content, created_at, updated_at, deleted_at
             FROM capsules WHERE id = ?1",
            params![resolved_id],
            row_to_capsule,
        )
        .context("failed to read revised capsule")?;

    tx.commit()?;
    Ok(Some(capsule))
}

/// Soft delete de una capsule.
///
/// Acepta UUID completo o prefijo ≥8 chars. Devuelve `None` si no existe
/// o ya estaba descartada.
pub fn discard(conn: &mut Connection, id: &str) -> Result<Option<CapsuleDiscardResult>> {
    let Some(resolved_id) = resolve_id(conn, "capsules", id)? else {
        return Ok(None);
    };

    let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;

    let affected = tx
        .execute(
            "UPDATE capsules SET deleted_at = datetime('now')
         WHERE id = ?1 AND deleted_at IS NULL",
            params![resolved_id],
        )
        .context("failed to discard capsule")?;

    if affected == 0 {
        return Ok(None);
    }

    let deleted_at: String = tx.query_row(
        "SELECT deleted_at FROM capsules WHERE id = ?1",
        params![resolved_id],
        |row| row.get(0),
    )?;

    tx.commit()?;
    Ok(Some(CapsuleDiscardResult { id: resolved_id, deleted_at }))
}

/// Lista capsules globales con filtros opcionales (incluye archivadas).
pub fn count(conn: &Connection) -> Result<u32> {
    let n: u32 = conn.query_row(
        "SELECT COUNT(*) FROM capsules",
        [],
        |r| r.get(0),
    )?;
    Ok(n)
}

/// Cuenta capsules archivadas (`deleted_at IS NOT NULL`).
///
/// Permite mostrar el trailer `... N más archivados` con el número exacto
/// de capsules ocultas tras aplicar el cap del CLI.
pub fn count_archived(conn: &Connection) -> Result<u32> {
    let n: u32 = conn.query_row(
        "SELECT COUNT(*) FROM capsules WHERE deleted_at IS NOT NULL",
        [],
        |r| r.get(0),
    )?;
    Ok(n)
}

pub fn list(
    conn: &Connection,
    filter: ListFilter,
    compound: Option<&str>,
    pagination: &PaginationParams,
) -> Result<Paginated<Capsule>> {
    let filter_clause = match filter {
        ListFilter::Active => "deleted_at IS NULL",
        ListFilter::Archived => "deleted_at IS NOT NULL",
        ListFilter::All => "1=1",
    };

    let count_sql = format!(
        "SELECT COUNT(*) FROM capsules
         WHERE {filter_clause} AND (?1 IS NULL OR compound = ?1)"
    );
    let total: u64 = conn.query_row(&count_sql, params![compound], |r| r.get(0))?;

    let limit = pagination.limit() as i64;
    let offset = pagination.offset() as i64;

    let select_sql = format!(
        "SELECT id, compound, title, content, created_at, updated_at, deleted_at
         FROM capsules
         WHERE {filter_clause} AND (?1 IS NULL OR compound = ?1)
         ORDER BY updated_at DESC
         LIMIT ?2 OFFSET ?3"
    );
    let mut stmt = conn.prepare(&select_sql)?;
    let items = stmt
        .query_map(params![compound, limit, offset], row_to_capsule)?
        .collect::<rusqlite::Result<Vec<_>>>()
        .context("failed to list capsules")?;

    Ok(Paginated {
        items,
        total,
        page: pagination.page,
        page_size: pagination.page_size,
    })
}

/// Hard delete de una capsule (irreversible).
///
/// Acepta UUID completo o prefijo ≥8 chars. Elimina la fila de `capsules`
/// permanentemente. Devuelve `Ok(true)` si existía y fue eliminada,
/// `Ok(false)` si no existía.
pub fn hard_delete(conn: &mut Connection, id: &str) -> Result<bool> {
    let Some(resolved_id) = resolve_id(conn, "capsules", id)? else {
        return Ok(false);
    };

    let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;

    // resolve_id puede hacer short-circuit con un UUID completo sin verificar
    // que exista, por lo que volvemos a verificar con `changes` del DELETE.
    let count = tx
        .execute("DELETE FROM capsules WHERE id = ?1", params![resolved_id])
        .context("failed to delete capsule")?;

    tx.commit()?;
    Ok(count > 0)
}

/// Compounds distintos usados en capsules, ordenados por frecuencia descendente.
///
/// Las capsules son globales (no pertenecen a ningún bottle), por lo que no
/// admite filtro por bottle. Devuelve `(compound, count)` con count DESC y
/// compound ASC como desempate estable.
pub fn distinct_compounds(conn: &Connection, limit: u32) -> Result<Vec<(String, i64)>> {
    let mut stmt = conn.prepare(
        "SELECT compound, COUNT(*) as c
         FROM capsules
         WHERE deleted_at IS NULL
         GROUP BY compound
         ORDER BY c DESC, compound ASC
         LIMIT ?1",
    )?;
    let rows = stmt
        .query_map(params![limit], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
        })?
        .collect::<rusqlite::Result<Vec<_>>>()
        .context("failed to load distinct capsule compounds")?;
    Ok(rows)
}

/// Mapea una fila de SQLite al tipo [`Capsule`].
fn row_to_capsule(row: &rusqlite::Row<'_>) -> rusqlite::Result<Capsule> {
    Ok(Capsule {
        id: row.get(0)?,
        compound: row.get(1)?,
        title: row.get(2)?,
        content: row.get(3)?,
        created_at: row.get(4)?,
        updated_at: row.get(5)?,
        deleted_at: row.get(6)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::connection::open_in_memory;
    use crate::domain::capsule::{CapsulePatch, NewCapsule};

    fn sample_capsule() -> NewCapsule {
        NewCapsule {
            title: "Snake_case siempre".into(),
            content: "Prefiero snake_case en todos los proyectos, incluso en TypeScript.".into(),
            compound: "convention".into(),
        }
    }

    #[test]
    fn take_and_read() {
        let mut conn = open_in_memory().unwrap();
        let input = sample_capsule();
        let result = take(&mut conn, &input).unwrap();
        assert_eq!(result.action, "created");
        assert_eq!(result.title, input.title);
        assert_eq!(result.compound, input.compound.as_str());
        assert_eq!(result.content, input.content);

        let cap = read(&conn, &result.id).unwrap().unwrap();
        assert_eq!(cap.compound, "convention");
        assert_eq!(cap.title, "Snake_case siempre");
    }

    #[test]
    fn revise_partial_patch() {
        let mut conn = open_in_memory().unwrap();
        let cap = take(&mut conn, &sample_capsule()).unwrap();

        let updated = revise(
            &mut conn,
            &cap.id,
            &CapsulePatch {
                title: Some("Convención de nombres".into()),
                content: None,
                compound: None,
            },
        )
        .unwrap()
        .unwrap();

        assert_eq!(updated.title, "Convención de nombres");
        assert!(updated.content.contains("snake_case"));
    }

    #[test]
    fn discard_soft_deletes() {
        let mut conn = open_in_memory().unwrap();
        let cap = take(&mut conn, &sample_capsule()).unwrap();
        let result = discard(&mut conn, &cap.id).unwrap().unwrap();
        assert_eq!(result.id, cap.id);
        assert!(read(&conn, &cap.id).unwrap().is_none());
    }

    #[test]
    fn discard_twice_returns_none() {
        let mut conn = open_in_memory().unwrap();
        let cap = take(&mut conn, &sample_capsule()).unwrap();
        discard(&mut conn, &cap.id).unwrap();
        assert!(discard(&mut conn, &cap.id).unwrap().is_none());
    }

    #[test]
    fn read_missing_returns_none() {
        let conn = open_in_memory().unwrap();
        assert!(read(&conn, "00000000-0000-0000-0000-000000000000").unwrap().is_none());
    }

    #[test]
    fn revise_missing_returns_none() {
        let mut conn = open_in_memory().unwrap();
        let result = revise(
            &mut conn,
            "00000000-0000-0000-0000-000000000000",
            &CapsulePatch {
                title: Some("x".into()),
                content: None,
                compound: None,
            },
        )
        .unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn revise_discarded_returns_none() {
        let mut conn = open_in_memory().unwrap();
        let cap = take(&mut conn, &sample_capsule()).unwrap();
        discard(&mut conn, &cap.id).unwrap();

        let result = revise(
            &mut conn,
            &cap.id,
            &CapsulePatch {
                title: Some("nuevo".into()),
                content: None,
                compound: None,
            },
        )
        .unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn list_returns_all_active() {
        let mut conn = open_in_memory().unwrap();
        take(&mut conn, &sample_capsule()).unwrap();
        take(
            &mut conn,
            &NewCapsule {
                title: "Otra convención".into(),
                content: "Usar PascalCase para tipos.".into(),
                compound: "convention".into(),
            },
        )
        .unwrap();

        let all = list(&conn, ListFilter::Active, None, &PaginationParams::default()).unwrap();
        assert_eq!(all.items.len(), 2);
        assert_eq!(all.total, 2);
    }

    #[test]
    fn list_filters_by_compound() {
        let mut conn = open_in_memory().unwrap();
        take(&mut conn, &sample_capsule()).unwrap(); // convention
        take(
            &mut conn,
            &NewCapsule {
                title: "Workflow de deploy".into(),
                content: "push a main = deploy automático.".into(),
                compound: "workflow".into(),
            },
        )
        .unwrap();

        let conventions = list(&conn, ListFilter::Active, Some("convention"), &PaginationParams::default()).unwrap();
        assert_eq!(conventions.items.len(), 1);
        assert_eq!(conventions.items[0].compound, "convention");
    }

    #[test]
    fn list_respects_limit() {
        let mut conn = open_in_memory().unwrap();
        for i in 0..5 {
            take(
                &mut conn,
                &NewCapsule {
                    title: format!("Cap {i}"),
                    content: "contenido".into(),
                    compound: "manual".into(),
                },
            )
            .unwrap();
        }

        let limited = list(&conn, ListFilter::Active, None, &PaginationParams { page: 1, page_size: 3 }).unwrap();
        assert_eq!(limited.items.len(), 3);
        assert_eq!(limited.total, 5);
    }

    #[test]
    fn list_excludes_discarded() {
        let mut conn = open_in_memory().unwrap();
        let cap = take(&mut conn, &sample_capsule()).unwrap();
        discard(&mut conn, &cap.id).unwrap();

        let all = list(&conn, ListFilter::Active, None, &PaginationParams::default()).unwrap();
        assert!(all.items.is_empty());
        assert_eq!(all.total, 0);
    }

    #[test]
    fn read_any_finds_archived() {
        let mut conn = open_in_memory().unwrap();
        let cap = take(&mut conn, &sample_capsule()).unwrap();
        discard(&mut conn, &cap.id).unwrap();

        let found = read_any(&conn, &cap.id).unwrap();
        assert!(found.is_some());
        assert!(found.unwrap().deleted_at.is_some());
    }

    #[test]
    fn read_excludes_archived_but_read_any_does_not() {
        let mut conn = open_in_memory().unwrap();
        let cap = take(&mut conn, &sample_capsule()).unwrap();
        discard(&mut conn, &cap.id).unwrap();

        assert!(read(&conn, &cap.id).unwrap().is_none());
        assert!(read_any(&conn, &cap.id).unwrap().is_some());
    }

    #[test]
    fn hard_delete_removes_row() {
        let mut conn = open_in_memory().unwrap();
        let cap = take(&mut conn, &sample_capsule()).unwrap();
        let deleted = hard_delete(&mut conn, &cap.id).unwrap();
        assert!(deleted);
        assert!(read_any(&conn, &cap.id).unwrap().is_none());
    }

    #[test]
    fn hard_delete_nonexistent_returns_false() {
        let mut conn = open_in_memory().unwrap();
        let deleted = hard_delete(&mut conn, "00000000-0000-0000-0000-000000000000").unwrap();
        assert!(!deleted);
    }

    #[test]
    fn list_active_only_excludes_archived() {
        let mut conn = open_in_memory().unwrap();
        for i in 0..3 {
            take(&mut conn, &NewCapsule { title: format!("Active {i}"), content: "c".into(), compound: "discovery".into() }).unwrap();
        }
        let archived_ids: Vec<String> = (0..5)
            .map(|i| take(&mut conn, &NewCapsule { title: format!("Archived {i}"), content: "c".into(), compound: "discovery".into() }).unwrap().id)
            .collect();
        for id in &archived_ids {
            discard(&mut conn, id).unwrap();
        }
        let active = list(&conn, ListFilter::Active, None, &PaginationParams { page: 1, page_size: 100 }).unwrap();
        assert_eq!(active.items.len(), 3);
        assert_eq!(active.total, 3);
        assert!(active.items.iter().all(|c| c.deleted_at.is_none()));
    }

    #[test]
    fn count_archived_still_returns_total() {
        let mut conn = open_in_memory().unwrap();
        let archived_ids: Vec<String> = (0..9)
            .map(|i| take(&mut conn, &NewCapsule { title: format!("Archived {i}"), content: "c".into(), compound: "discovery".into() }).unwrap().id)
            .collect();
        for id in &archived_ids {
            discard(&mut conn, id).unwrap();
        }
        let total = count_archived(&conn).unwrap();
        assert_eq!(total, 9);
    }

    #[test]
    fn read_by_12char_prefix() {
        let mut conn = open_in_memory().unwrap();
        let result = take(&mut conn, &sample_capsule()).unwrap();
        let short = result.id.replace('-', "").chars().take(12).collect::<String>();
        let found = read(&conn, &short).unwrap().unwrap();
        assert_eq!(found.id, result.id);
    }

    #[test]
    fn revise_by_short_id() {
        let mut conn = open_in_memory().unwrap();
        let result = take(&mut conn, &sample_capsule()).unwrap();
        let short = result.id.replace('-', "").chars().take(12).collect::<String>();
        let updated = revise(
            &mut conn,
            &short,
            &CapsulePatch { title: Some("Revisada por prefijo".into()), content: None, compound: None },
        )
        .unwrap()
        .unwrap();
        assert_eq!(updated.title, "Revisada por prefijo");
    }

    #[test]
    fn discard_by_short_id() {
        let mut conn = open_in_memory().unwrap();
        let result = take(&mut conn, &sample_capsule()).unwrap();
        let short = result.id.replace('-', "").chars().take(12).collect::<String>();
        let discarded = discard(&mut conn, &short).unwrap().unwrap();
        assert_eq!(discarded.id, result.id);
        assert!(read(&conn, &result.id).unwrap().is_none());
    }

    #[test]
    fn read_too_short_returns_invalid_id() {
        use crate::error::PillboxError;
        let conn = open_in_memory().unwrap();
        let err = read(&conn, "abc").unwrap_err();
        let typed = err.downcast_ref::<PillboxError>().unwrap();
        assert!(matches!(typed, PillboxError::InvalidId { .. }));
    }

    #[test]
    fn distinct_compounds_orders_count_desc_compound_asc_excludes_deleted() {
        let mut conn = open_in_memory().unwrap();

        // 2x "convention", 1x "workflow", 1x "discovery" (será soft-deleted).
        take(
            &mut conn,
            &NewCapsule {
                title: "c1".into(),
                content: "x".into(),
                compound: "convention".into(),
            },
        )
        .unwrap();
        take(
            &mut conn,
            &NewCapsule {
                title: "c2".into(),
                content: "x".into(),
                compound: "convention".into(),
            },
        )
        .unwrap();
        take(
            &mut conn,
            &NewCapsule {
                title: "c3".into(),
                content: "x".into(),
                compound: "workflow".into(),
            },
        )
        .unwrap();
        let deleted = take(
            &mut conn,
            &NewCapsule {
                title: "c4".into(),
                content: "x".into(),
                compound: "discovery".into(),
            },
        )
        .unwrap();
        discard(&mut conn, &deleted.id).unwrap();

        let rows = distinct_compounds(&conn, 50).unwrap();
        let convention = rows.iter().find(|(c, _)| c == "convention").unwrap();
        let workflow = rows.iter().find(|(c, _)| c == "workflow").unwrap();
        assert_eq!(convention.1, 2);
        assert_eq!(workflow.1, 1);
        assert!(
            !rows.iter().any(|(c, _)| c == "discovery"),
            "compound borrado no debe aparecer"
        );

        let i_convention = rows.iter().position(|(c, _)| c == "convention").unwrap();
        let i_workflow = rows.iter().position(|(c, _)| c == "workflow").unwrap();
        assert!(i_convention < i_workflow, "convention (2) antes que workflow (1)");
    }

    #[test]
    fn distinct_compounds_tiebreak_alphabetical() {
        let mut conn = open_in_memory().unwrap();
        for compound in ["zeta", "alfa", "zeta", "alfa"] {
            take(
                &mut conn,
                &NewCapsule {
                    title: "t".into(),
                    content: "c".into(),
                    compound: compound.into(),
                },
            )
            .unwrap();
        }
        let rows = distinct_compounds(&conn, 50).unwrap();
        let i_alfa = rows.iter().position(|(c, _)| c == "alfa").unwrap();
        let i_zeta = rows.iter().position(|(c, _)| c == "zeta").unwrap();
        assert!(i_alfa < i_zeta);
    }

    #[test]
    fn read_ambiguous_returns_ambiguous_id() {
        use crate::error::PillboxError;
        use rusqlite::params;
        let mut conn = open_in_memory().unwrap();
        conn.execute(
            "INSERT INTO capsules (id, compound, title, content)
             VALUES ('01234567-aaaa-7000-8000-000000000001', 'convention', 'A', 'c')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO capsules (id, compound, title, content)
             VALUES ('01234567-aaaa-7000-8000-000000000002', 'convention', 'B', 'c')",
            [],
        )
        .unwrap();
        let err = read(&conn, "01234567aaaa").unwrap_err();
        let typed = err.downcast_ref::<PillboxError>().unwrap();
        assert!(matches!(typed, PillboxError::AmbiguousId { .. }));
    }
}
