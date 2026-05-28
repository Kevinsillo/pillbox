//! Funciones de contexto navegable: índices de prescriptions y pills.
//!
//! Sin FTS5 ni fuzzy — sirven para navegar la jerarquía bottle → prescription → pill
//! sin escribir una query de texto. Para búsqueda por término ver [`super::search`].

use anyhow::{Context, Result};
use rusqlite::{params, Connection};

use crate::db::store::id_resolver::normalize_prefix;

pub struct BottleRxEntry {
    pub id: String,
    pub title: String,
    pub started_at: String,
    pub ended_at: Option<String>,
    pub pill_count: i64,
}

pub struct BottleContextResult {
    pub prescriptions: Vec<BottleRxEntry>,
    /// Total de prescriptions activas en el bottle (COUNT en DB, no `prescriptions.len()`).
    pub prescription_count: u64,
}

pub struct PrescriptionPillEntry {
    pub id: String,
    pub compound: String,
    pub title: String,
    pub snippet: String,
}

pub struct PrescriptionContextResult {
    pub id: Option<String>,
    pub title: String,
    pub started_at: String,
    pub ended_at: Option<String>,
    pub pills: Vec<PrescriptionPillEntry>,
    /// Total de pills activas en la prescription (COUNT en DB, no `pills.len()`).
    pub pill_count: u64,
}

/// Índice navegable de prescriptions de un bottle.
///
/// Devuelve las `limit` prescriptions más recientes con id, título, estado,
/// fechas y pill_count. Sin contenido de pills — usar `prescription_context`
/// para profundizar en una prescription concreta.
pub fn bottle_context(
    conn: &Connection,
    bottle_id: &str,
    limit: u32,
) -> Result<BottleContextResult> {
    let normalized_bottle = normalize_prefix(bottle_id);
    let bottle_pattern = format!("{}%", normalized_bottle);

    let mut stmt = conn.prepare(
        "SELECT rx.id, rx.title, rx.started_at, rx.ended_at,
            COUNT(p.id) AS pill_count
        FROM prescriptions rx
        LEFT JOIN pills p ON p.prescription_id = rx.id AND p.deleted_at IS NULL
        WHERE (rx.bottle_id = ?1 OR rx.bottle_id LIKE ?2) AND rx.deleted_at IS NULL
        GROUP BY rx.id
        ORDER BY rx.started_at DESC
        LIMIT ?3",
    )?;

    let prescriptions: Vec<BottleRxEntry> = stmt
        .query_map(params![normalized_bottle, bottle_pattern, limit], |row| {
            Ok(BottleRxEntry {
                id: row.get(0)?,
                title: row.get(1)?,
                started_at: row.get(2)?,
                ended_at: row.get(3)?,
                pill_count: row.get(4)?,
            })
        })?
        .collect::<rusqlite::Result<Vec<_>>>()
        .context("failed to load prescriptions for bottle_context")?;

    // Total real en DB — no `prescriptions.len()` (limitado por `limit`).
    let prescription_count =
        crate::db::store::prescriptions::count_active_by_bottle(conn, &normalized_bottle)?;
    Ok(BottleContextResult {
        prescriptions,
        prescription_count,
    })
}

/// Pills de una prescription concreta, ordenadas de más reciente a más antigua.
///
/// Devuelve hasta `limit` pills con id, compound, título y snippet de 300 chars.
/// Para el contenido completo de una pill individual usar `pill_read`.
pub fn prescription_context(
    conn: &Connection,
    prescription_id: &str,
    limit: u32,
) -> Result<PrescriptionContextResult> {
    let row = conn.query_row(
        "SELECT id, title, started_at, ended_at
        FROM prescriptions
        WHERE (id = ?1 OR id LIKE ?2) AND deleted_at IS NULL
        ORDER BY started_at DESC LIMIT 1",
        params![
            normalize_prefix(prescription_id),
            format!("{}%", normalize_prefix(prescription_id))
        ],
        |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, Option<String>>(3)?,
            ))
        },
    );

    let (rx_id, rx_title, rx_started, rx_ended) = match row {
        Ok(r) => r,
        Err(rusqlite::Error::QueryReturnedNoRows) => {
            return Ok(PrescriptionContextResult {
                id: None,
                title: String::new(),
                started_at: String::new(),
                ended_at: None,
                pills: vec![],
                pill_count: 0,
            });
        }
        Err(e) => return Err(e).context("failed to read prescription"),
    };

    let mut pill_stmt = conn.prepare(
        "SELECT id, compound, title, content
        FROM pills
        WHERE prescription_id = ?1 AND deleted_at IS NULL
        ORDER BY created_at DESC
        LIMIT ?2",
    )?;

    let pills: Vec<PrescriptionPillEntry> = pill_stmt
        .query_map(params![rx_id, limit], |row| {
            let content: String = row.get(3)?;
            let flat: String = content.replace("\r\n", "\\n").replace(['\n', '\r'], "\\n");
            let chars: Vec<char> = flat.chars().collect();
            let snippet = if chars.len() > 300 {
                format!("{}…", chars[..299].iter().collect::<String>())
            } else {
                flat
            };
            Ok(PrescriptionPillEntry {
                id: row.get(0)?,
                compound: row.get(1)?,
                title: row.get(2)?,
                snippet,
            })
        })?
        .collect::<rusqlite::Result<Vec<_>>>()
        .context("failed to load pills for prescription_context")?;

    // Total real en DB — no `pills.len()` (limitado por `limit`).
    let pill_count = crate::db::store::pills::count_active_by_prescription(conn, &rx_id)?;
    Ok(PrescriptionContextResult {
        id: Some(rx_id),
        title: rx_title,
        started_at: rx_started,
        ended_at: rx_ended,
        pills,
        pill_count,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::connection::open_in_memory;
    use crate::db::store::{bottles, pills, prescriptions};
    use crate::db::DbScope;
    use crate::domain::bottle::{BottleScope, NewBottle};
    use crate::domain::pill::NewPill;
    use crate::domain::prescription::NewPrescription;

    fn setup_with_pills(conn: &mut Connection) -> String {
        let bottle = bottles::create(
            conn,
            &NewBottle {
                name: "search-test".into(),
                display_name: "Search Test".into(),
                directory: "/tmp/search-test".into(),
                scope: BottleScope::Local,
            },
        )
        .unwrap();

        let rx = prescriptions::open(
            conn,
            &NewPrescription {
                bottle_id: bottle.id.clone(),
                title: "Sesión de búsqueda".into(),
                author_name: None,
                author_email: None,
            },
        )
        .unwrap();

        for (title, content, compound) in [
            (
                "JWT tokens con refresh",
                "Implementamos stateless JWT con refresh tokens almacenados en SQLite.",
                "decision",
            ),
            (
                "Race condition en dedup",
                "BEGIN IMMEDIATE previene race conditions en escrituras concurrentes.",
                "bugfix",
            ),
            (
                "FTS5 tokenizer unicode61",
                "El tokenizer unicode61 normaliza acentos automáticamente.",
                "discovery",
            ),
        ] {
            pills::take(
                conn,
                &NewPill {
                    title: title.into(),
                    content: content.into(),
                    compound: compound.into(),
                    prescription_id: rx.id.clone(),
                    author_name: None,
                    author_email: None,
                },
            )
            .unwrap();
        }

        bottle.id
    }

    #[test]
    fn bottle_context_lists_prescriptions() {
        let mut conn = open_in_memory(DbScope::Global).unwrap();
        let bottle_id = setup_with_pills(&mut conn);

        let ctx = bottle_context(&conn, &bottle_id, 30).unwrap();
        assert!(ctx.prescription_count > 0);
        assert!(!ctx.prescriptions.is_empty());
        assert!(!ctx.prescriptions[0].id.is_empty());
        assert!(ctx.prescriptions[0].pill_count >= 0);
    }

    #[test]
    fn bottle_context_resolves_12char_prefix() {
        let mut conn = open_in_memory(DbScope::Global).unwrap();
        let bottle_id = setup_with_pills(&mut conn);
        let short = bottle_id
            .replace('-', "")
            .chars()
            .take(12)
            .collect::<String>();

        let ctx = bottle_context(&conn, &short, 30).unwrap();
        assert!(
            ctx.prescription_count > 0,
            "debe encontrar prescriptions con prefijo de 12 chars"
        );
    }

    #[test]
    fn bottle_context_resolves_8char_prefix() {
        let mut conn = open_in_memory(DbScope::Global).unwrap();
        let bottle_id = setup_with_pills(&mut conn);
        let short = bottle_id
            .replace('-', "")
            .chars()
            .take(8)
            .collect::<String>();

        let ctx = bottle_context(&conn, &short, 30).unwrap();
        assert!(
            ctx.prescription_count > 0,
            "debe encontrar prescriptions con prefijo de 8 chars"
        );
    }

    #[test]
    fn bottle_context_empty_bottle_returns_empty() {
        let mut conn = open_in_memory(DbScope::Global).unwrap();
        let bottle = bottles::create(
            &mut conn,
            &NewBottle {
                name: "vacio".into(),
                display_name: "Vacío".into(),
                directory: "/tmp/vacio".into(),
                scope: BottleScope::Local,
            },
        )
        .unwrap();

        let ctx = bottle_context(&conn, &bottle.id, 30).unwrap();
        assert_eq!(ctx.prescription_count, 0);
        assert!(ctx.prescriptions.is_empty());
    }

    #[test]
    fn prescription_context_returns_pills() {
        let mut conn = open_in_memory(DbScope::Global).unwrap();
        let bottle_id = setup_with_pills(&mut conn);

        let rx_id: String = conn
            .query_row(
                "SELECT id FROM prescriptions WHERE bottle_id = ?1 LIMIT 1",
                params![bottle_id],
                |r| r.get(0),
            )
            .unwrap();

        let ctx = prescription_context(&conn, &rx_id, 30).unwrap();
        assert!(ctx.id.is_some());
        assert!(ctx.pill_count > 0);
        assert!(!ctx.pills.is_empty());
        assert!(!ctx.pills[0].id.is_empty());
        let compounds = ["decision", "bugfix", "discovery"];
        assert!(compounds.contains(&ctx.pills[0].compound.as_str()));
    }

    #[test]
    fn prescription_context_resolves_12char_prefix() {
        let mut conn = open_in_memory(DbScope::Global).unwrap();
        let bottle_id = setup_with_pills(&mut conn);

        let rx_id: String = conn
            .query_row(
                "SELECT id FROM prescriptions WHERE bottle_id = ?1 LIMIT 1",
                params![bottle_id],
                |r| r.get(0),
            )
            .unwrap();

        let short = rx_id.replace('-', "").chars().take(12).collect::<String>();
        let ctx = prescription_context(&conn, &short, 30).unwrap();
        assert!(
            ctx.id.is_some(),
            "debe resolver la prescription con prefijo de 12 chars"
        );
        assert!(ctx.pill_count > 0);
    }

    #[test]
    fn prescription_context_truncates_multibyte_safely() {
        let mut conn = open_in_memory(DbScope::Global).unwrap();
        let bottle = bottles::create(
            &mut conn,
            &NewBottle {
                name: "utf8-test".into(),
                display_name: "UTF-8 Test".into(),
                directory: "/tmp/utf8-test".into(),
                scope: BottleScope::Local,
            },
        )
        .unwrap();
        let rx = prescriptions::open(
            &mut conn,
            &NewPrescription {
                bottle_id: bottle.id.clone(),
                title: "rx utf8".into(),
                author_name: None,
                author_email: None,
            },
        )
        .unwrap();
        let long_content = format!("{}{}", "a".repeat(296), "🦀".repeat(10));
        pills::take(
            &mut conn,
            &NewPill {
                title: "pill con emoji".into(),
                content: long_content,
                compound: "discovery".into(),
                prescription_id: rx.id.clone(),
                author_name: None,
                author_email: None,
            },
        )
        .unwrap();

        let result = prescription_context(&conn, &rx.id, 30);
        assert!(result.is_ok());
        let ctx = result.unwrap();
        assert!(!ctx.pills.is_empty());
        assert!(ctx.pills[0].snippet.contains("🦀") || ctx.pills[0].snippet.contains("…"));
    }

    #[test]
    fn prescription_context_unknown_id_returns_empty() {
        let conn = open_in_memory(DbScope::Global).unwrap();
        let ctx = prescription_context(&conn, "00000000-0000-0000-0000-000000000000", 30).unwrap();
        assert!(ctx.id.is_none());
        assert!(ctx.pills.is_empty());
    }
}
