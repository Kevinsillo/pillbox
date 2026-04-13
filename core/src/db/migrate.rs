//! Migración de pills/capsules entre dos bases de datos.
//!
//! Usado por `pillbox bottle migrate` para mover el conocimiento de un
//! bottle entre la DB local (`.pillbox/pillbox.db`) y la global (`~/.pillbox/pillbox.db`).
//!
//! La estrategia es upsert por `sync_id`:
//!   - Si el registro ya existe en destino, se actualiza solo si `updated_at` es más reciente.
//!   - Si no existe, se inserta.
//!   - Los registros descartados (`deleted_at IS NOT NULL`) se migran tal cual.

use anyhow::{Context, Result};
use rusqlite::{params, Connection};

#[derive(Debug)]
pub struct MigrateResult {
    pub bottles:       usize,
    pub prescriptions: usize,
    pub pills:         usize,
    pub capsules:      usize,
}

/// Migra un bottle completo (prescripciones + pills) de `src` a `dst`.
///
/// `bottle_name` es el slug del bottle (columna `bottles.name`).
/// Las capsules son globales — no tienen bottle, se pasan separadas con `include_capsules`.
pub fn migrate_bottle(
    src: &Connection,
    dst: &mut Connection,
    bottle_name: &str,
    include_capsules: bool,
) -> Result<MigrateResult> {
    // ── 1. Bottle ─────────────────────────────────────────────────────────────
    let bottle = src
        .query_row(
            "SELECT name, display_name, directory, scope FROM bottles WHERE name = ?1",
            params![bottle_name],
            |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?, r.get::<_, String>(2)?, r.get::<_, String>(3)?)),
        )
        .with_context(|| format!("bottle '{}' no encontrado en origen", bottle_name))?;

    let tx = dst.transaction()?;

    tx.execute(
        "INSERT INTO bottles (name, display_name, directory, scope)
         VALUES (?1, ?2, ?3, ?4)
         ON CONFLICT(name) DO UPDATE SET
             display_name = excluded.display_name,
             directory    = excluded.directory,
             last_seen_at = datetime('now')",
        params![bottle.0, bottle.1, bottle.2, bottle.3],
    )
    .context("no se pudo upsert el bottle en destino")?;

    let dst_bottle_id: i64 = tx.query_row(
        "SELECT id FROM bottles WHERE name = ?1",
        params![bottle_name],
        |r| r.get(0),
    )?;

    // ── 2. Prescripciones ─────────────────────────────────────────────────────
    let src_bottle_id: i64 = src.query_row(
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
            "SELECT sync_id, compound, title, content, prescription_id,
                    dispenser, author_name, author_email,
                    created_at, updated_at, deleted_at
             FROM pills WHERE prescription_id = ?1",
        )?;

        let pills: Vec<_> = pill_stmt
            .query_map(params![rx_id], |r| {
                Ok((
                    r.get::<_, String>(0)?,   // sync_id
                    r.get::<_, String>(1)?,   // compound
                    r.get::<_, String>(2)?,   // title
                    r.get::<_, String>(3)?,   // content
                    r.get::<_, String>(4)?,   // prescription_id
                    r.get::<_, Option<String>>(5)?, // dispenser
                    r.get::<_, Option<String>>(6)?, // author_name
                    r.get::<_, Option<String>>(7)?, // author_email
                    r.get::<_, String>(8)?,   // created_at
                    r.get::<_, String>(9)?,   // updated_at
                    r.get::<_, Option<String>>(10)?, // deleted_at
                ))
            })?
            .collect::<rusqlite::Result<_>>()?;

        pill_count += pills.len();

        for p in pills {
            tx.execute(
                "INSERT INTO pills
                     (sync_id, compound, title, content, prescription_id,
                      dispenser, author_name, author_email, created_at, updated_at, deleted_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)
                 ON CONFLICT(sync_id) DO UPDATE SET
                     title      = excluded.title,
                     content    = excluded.content,
                     compound   = excluded.compound,
                     updated_at = excluded.updated_at,
                     deleted_at = excluded.deleted_at
                 WHERE excluded.updated_at > pills.updated_at",
                params![p.0, p.1, p.2, p.3, p.4, p.5, p.6, p.7, p.8, p.9, p.10],
            )?;
        }
    }

    // ── 4. Capsules (opcionales — son globales) ───────────────────────────────
    let capsule_count = if include_capsules {
        let mut cap_stmt = src.prepare(
            "SELECT sync_id, compound, title, content, created_at, updated_at, deleted_at
             FROM capsules",
        )?;

        let capsules: Vec<_> = cap_stmt
            .query_map([], |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, String>(2)?,
                    r.get::<_, String>(3)?,
                    r.get::<_, String>(4)?,
                    r.get::<_, String>(5)?,
                    r.get::<_, Option<String>>(6)?,
                ))
            })?
            .collect::<rusqlite::Result<_>>()?;

        let n = capsules.len();
        for c in capsules {
            tx.execute(
                "INSERT INTO capsules
                     (sync_id, compound, title, content, created_at, updated_at, deleted_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
                 ON CONFLICT(sync_id) DO UPDATE SET
                     title      = excluded.title,
                     content    = excluded.content,
                     compound   = excluded.compound,
                     updated_at = excluded.updated_at,
                     deleted_at = excluded.deleted_at
                 WHERE excluded.updated_at > capsules.updated_at",
                params![c.0, c.1, c.2, c.3, c.4, c.5, c.6],
            )?;
        }
        n
    } else {
        0
    };

    tx.commit()?;

    Ok(MigrateResult {
        bottles:       1,
        prescriptions: rx_count,
        pills:         pill_count,
        capsules:      capsule_count,
    })
}
