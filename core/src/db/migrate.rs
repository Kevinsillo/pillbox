//! Migración de un bottle entre dos bases de datos (operación de corte, no copia).
//!
//! Usado por `pillbox bottle migrate` para mover el conocimiento de un
//! bottle entre la DB local (`.pillbox/pillbox.db`) y la global (`~/.pillbox/pillbox.db`).
//!
//! La estrategia es upsert por `id` (UUID v7):
//!   - Si el registro ya existe en destino, se actualiza solo si `updated_at` es más reciente.
//!   - Si no existe, se inserta.
//!   - Los registros descartados (`deleted_at IS NOT NULL`) se migran tal cual.
//!
//! Tras la migración, el origen debe eliminarse (ver `delete_bottle`).

use anyhow::{Context, Result};
use rusqlite::{params, Connection};

#[derive(Debug)]
pub struct MigrateResult {
    pub bottles: usize,
    pub prescriptions: usize,
    pub pills: usize,
}

/// Migra un bottle completo (prescripciones + pills) de `src` a `dst`.
///
/// `bottle_name` es el slug del bottle (columna `bottles.name`).
pub fn migrate_bottle(
    src: &Connection,
    dst: &mut Connection,
    bottle_name: &str,
) -> Result<MigrateResult> {
    // ── 1. Bottle ─────────────────────────────────────────────────────────────
    let bottle = src
        .query_row(
            "SELECT id, name, display_name, directory, scope FROM bottles WHERE name = ?1",
            params![bottle_name],
            |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, String>(2)?,
                    r.get::<_, String>(3)?,
                    r.get::<_, String>(4)?,
                ))
            },
        )
        .with_context(|| format!("bottle '{}' not found in source", bottle_name))?;

    let tx = dst.transaction()?;

    tx.execute(
        "INSERT INTO bottles (id, name, display_name, directory, scope)
         VALUES (?1, ?2, ?3, ?4, ?5)
         ON CONFLICT(name) DO UPDATE SET
             display_name = excluded.display_name,
             directory    = excluded.directory,
             last_seen_at = datetime('now')",
        params![bottle.0, bottle.1, bottle.2, bottle.3, bottle.4],
    )
    .context("failed to upsert bottle in destination")?;

    let dst_bottle_id: String = tx.query_row(
        "SELECT id FROM bottles WHERE name = ?1",
        params![bottle_name],
        |r| r.get(0),
    )?;

    // ── 2. Prescripciones ─────────────────────────────────────────────────────
    let src_bottle_id: String = src.query_row(
        "SELECT id FROM bottles WHERE name = ?1",
        params![bottle_name],
        |r| r.get(0),
    )?;

    let mut rx_stmt = src.prepare(
        "SELECT id, title, started_at, ended_at, deleted_at
         FROM prescriptions WHERE bottle_id = ?1",
    )?;

    let prescriptions: Vec<(String, String, String, Option<String>, Option<String>)> = rx_stmt
        .query_map(params![src_bottle_id], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
                r.get::<_, Option<String>>(3)?,
                r.get::<_, Option<String>>(4)?,
            ))
        })?
        .collect::<rusqlite::Result<_>>()?;

    let rx_count = prescriptions.len();

    for (id, title, started_at, ended_at, deleted_at) in &prescriptions {
        tx.execute(
            "INSERT INTO prescriptions (id, bottle_id, title, started_at, ended_at, deleted_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)
             ON CONFLICT(id) DO UPDATE SET
                 ended_at   = COALESCE(excluded.ended_at, prescriptions.ended_at),
                 deleted_at = COALESCE(excluded.deleted_at, prescriptions.deleted_at)",
            params![id, dst_bottle_id, title, started_at, ended_at, deleted_at],
        )?;
    }

    // ── 3. Pills ──────────────────────────────────────────────────────────────
    let rx_ids: Vec<String> = prescriptions.iter().map(|(id, ..)| id.clone()).collect();
    let mut pill_count = 0;

    for rx_id in &rx_ids {
        let mut pill_stmt = src.prepare(
            "SELECT id, compound, title, content, prescription_id,
                    author_name, author_email,
                    created_at, updated_at, deleted_at
             FROM pills WHERE prescription_id = ?1",
        )?;

        let pills: Vec<_> = pill_stmt
            .query_map(params![rx_id], |r| {
                Ok((
                    r.get::<_, String>(0)?,          // id
                    r.get::<_, String>(1)?,          // compound
                    r.get::<_, String>(2)?,          // title
                    r.get::<_, String>(3)?,          // content
                    r.get::<_, String>(4)?,          // prescription_id
                    r.get::<_, Option<String>>(5)?,  // author_name
                    r.get::<_, Option<String>>(6)?,  // author_email
                    r.get::<_, String>(7)?,          // created_at
                    r.get::<_, String>(8)?,          // updated_at
                    r.get::<_, Option<String>>(9)?,  // deleted_at
                ))
            })?
            .collect::<rusqlite::Result<_>>()?;

        pill_count += pills.len();

        for p in pills {
            tx.execute(
                "INSERT INTO pills
                     (id, compound, title, content, prescription_id,
                      author_name, author_email, created_at, updated_at, deleted_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
                 ON CONFLICT(id) DO UPDATE SET
                     title      = excluded.title,
                     content    = excluded.content,
                     compound   = excluded.compound,
                     updated_at = excluded.updated_at,
                     deleted_at = excluded.deleted_at
                 WHERE excluded.updated_at > pills.updated_at",
                params![p.0, p.1, p.2, p.3, p.4, p.5, p.6, p.7, p.8, p.9],
            )?;
        }
    }

    tx.commit()?;

    Ok(MigrateResult {
        bottles: 1,
        prescriptions: rx_count,
        pills: pill_count,
    })
}

/// Cuenta los contenidos de un bottle: (prescriptions activas, pills activas).
pub fn count_bottle_contents(conn: &Connection, bottle_name: &str) -> Result<(usize, usize)> {
    let bottle_id: String = conn.query_row(
        "SELECT id FROM bottles WHERE name = ?1",
        params![bottle_name],
        |r| r.get(0),
    )?;

    let prescriptions: usize = conn.query_row(
        "SELECT COUNT(*) FROM prescriptions WHERE bottle_id = ?1 AND deleted_at IS NULL",
        params![bottle_id],
        |r| r.get(0),
    )?;

    let pills: usize = conn.query_row(
        "SELECT COUNT(*) FROM pills p
         JOIN prescriptions rx ON p.prescription_id = rx.id
         WHERE rx.bottle_id = ?1 AND p.deleted_at IS NULL",
        params![bottle_id],
        |r| r.get(0),
    )?;

    Ok((prescriptions, pills))
}

/// Lista todos los bottles de una DB con sus conteos: Vec<(name, directory, pill_count)>.
pub fn list_bottles_with_counts(conn: &Connection) -> Result<Vec<(String, String, usize)>> {
    let mut stmt = conn.prepare(
        "SELECT b.name, b.directory,
                (SELECT COUNT(*) FROM pills p
                 JOIN prescriptions rx ON p.prescription_id = rx.id
                 WHERE rx.bottle_id = b.id AND p.deleted_at IS NULL)
         FROM bottles b
         ORDER BY b.name",
    )?;

    let rows = stmt
        .query_map([], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, usize>(2)?,
            ))
        })?
        .collect::<rusqlite::Result<_>>()?;

    Ok(rows)
}

/// Elimina un bottle y todos sus datos de una DB (prescripciones + pills + bottle).
///
/// Usado tras `migrate_bottle` para completar el corte en la DB origen.
pub fn delete_bottle(conn: &mut Connection, bottle_name: &str) -> Result<()> {
    let tx = conn.transaction()?;

    let bottle_id: String = tx
        .query_row(
            "SELECT id FROM bottles WHERE name = ?1",
            params![bottle_name],
            |r| r.get(0),
        )
        .with_context(|| format!("bottle '{}' not found for deletion", bottle_name))?;

    tx.execute(
        "DELETE FROM pills WHERE prescription_id IN (
             SELECT id FROM prescriptions WHERE bottle_id = ?1
         )",
        params![bottle_id],
    )?;

    tx.execute(
        "DELETE FROM prescriptions WHERE bottle_id = ?1",
        params![bottle_id],
    )?;
    tx.execute("DELETE FROM bottles WHERE id = ?1", params![bottle_id])?;

    tx.commit()?;
    Ok(())
}
