use anyhow::{Context, Result};
use rusqlite::{params, Connection};

use crate::domain::search::{SearchParams, SearchResult};

// ─── Sanitización de queries FTS5 ────────────────────────────────────────────

/// Convierte una query de texto libre en una expresión FTS5 segura.
///
/// Cada término se envuelve en comillas dobles para evitar que caracteres
/// especiales (AND, OR, NOT, *, etc.) sean interpretados como operadores.
/// El resultado es una búsqueda AND implícita de todos los términos.
///
/// Ejemplo: `"fix auth bug"` → `"fix" "auth" "bug"`
fn sanitize_fts_query(query: &str) -> String {
    query
        .split_whitespace()
        .map(|term| format!("\"{}\"", term.replace('"', "")))
        .collect::<Vec<_>>()
        .join(" ")
}

// ─── Pills ────────────────────────────────────────────────────────────────────

/// Busca pills por texto completo (FTS5).
///
/// Soporta filtros opcionales por bottle y compound.
/// El snippet se genera del campo `content` con marcadores `<b>`/`</b>`.
pub fn pill_find(conn: &Connection, params_in: &SearchParams) -> Result<Vec<SearchResult>> {
    let fts_query = sanitize_fts_query(&params_in.query);
    if fts_query.is_empty() {
        return Ok(vec![]);
    }

    let limit = params_in.limit.unwrap_or(20).min(100) as i64;

    // La condición de bottle_id requiere join con prescriptions.
    // La usamos siempre (LEFT JOIN) para poder filtrar opcionalmente.
    let mut stmt = conn.prepare(
        "SELECT p.id,
                p.sync_id,
                p.compound,
                p.title,
                snippet(pills_fts, 1, '<b>', '</b>', '...', 12) AS snippet,
                p.updated_at,
                pills_fts.rank
         FROM pills_fts
         JOIN pills p ON pills_fts.rowid = p.id
         LEFT JOIN prescriptions rx ON p.prescription_id = rx.id
         WHERE pills_fts MATCH ?1
           AND p.deleted_at IS NULL
           AND (?2 IS NULL OR rx.bottle_id = ?2)
           AND (?3 IS NULL OR p.compound = ?3)
         ORDER BY pills_fts.rank
         LIMIT ?4",
    )?;

    let results = stmt
        .query_map(
            params![fts_query, params_in.bottle_id, params_in.compound, limit],
            row_to_search_result,
        )?
        .collect::<rusqlite::Result<Vec<_>>>()
        .context("error en búsqueda FTS5 de pills")?;

    Ok(results)
}

// ─── Capsules ─────────────────────────────────────────────────────────────────

/// Busca capsules por texto completo (FTS5).
///
/// No soporta filtro por bottle (las capsules son globales del usuario).
pub fn capsule_find(
    conn: &Connection,
    query: &str,
    compound: Option<&str>,
    limit: Option<u32>,
) -> Result<Vec<SearchResult>> {
    let fts_query = sanitize_fts_query(query);
    if fts_query.is_empty() {
        return Ok(vec![]);
    }

    let limit = limit.unwrap_or(20).min(100) as i64;

    let mut stmt = conn.prepare(
        "SELECT c.id,
                c.sync_id,
                c.compound,
                c.title,
                snippet(capsules_fts, 1, '<b>', '</b>', '...', 12) AS snippet,
                c.updated_at,
                capsules_fts.rank
         FROM capsules_fts
         JOIN capsules c ON capsules_fts.rowid = c.id
         WHERE capsules_fts MATCH ?1
           AND c.deleted_at IS NULL
           AND (?2 IS NULL OR c.compound = ?2)
         ORDER BY capsules_fts.rank
         LIMIT ?3",
    )?;

    let results = stmt
        .query_map(params![fts_query, compound, limit], row_to_search_result)?
        .collect::<rusqlite::Result<Vec<_>>>()
        .context("error en búsqueda FTS5 de capsules")?;

    Ok(results)
}

// ─── Context ──────────────────────────────────────────────────────────────────

/// Resultado de `pill_context`: Markdown formateado listo para incluir en el
/// contexto del agente al iniciar una prescription.
pub struct ContextResult {
    pub context: String,
    pub prescription_count: usize,
    pub pill_count: usize,
}

/// Genera el contexto de un bottle para cargar al inicio de una sesión.
///
/// Devuelve las últimas `prescription_limit` prescriptions con su conteo de pills
/// y las últimas `pill_limit` pills, formateadas como Markdown.
pub fn pill_context(
    conn: &Connection,
    bottle_id: i64,
    prescription_limit: u32,
    pill_limit: u32,
) -> Result<ContextResult> {
    // Últimas N prescriptions con conteo de pills
    let mut rx_stmt = conn.prepare(
        "SELECT rx.id, rx.title, rx.started_at, rx.ended_at,
                COUNT(p.id) AS pill_count
         FROM prescriptions rx
         LEFT JOIN pills p ON p.prescription_id = rx.id AND p.deleted_at IS NULL
         WHERE rx.bottle_id = ?1 AND rx.deleted_at IS NULL
         GROUP BY rx.id
         ORDER BY rx.started_at DESC
         LIMIT ?2",
    )?;

    struct RxEntry {
        title: String,
        started_at: String,
        ended_at: Option<String>,
        pill_count: i64,
    }

    let prescriptions: Vec<RxEntry> = rx_stmt
        .query_map(params![bottle_id, prescription_limit], |row| {
            Ok(RxEntry {
                title: row.get(1)?,
                started_at: row.get(2)?,
                ended_at: row.get(3)?,
                pill_count: row.get(4)?,
            })
        })?
        .collect::<rusqlite::Result<Vec<_>>>()
        .context("error al cargar prescriptions para contexto")?;

    // Últimas M pills del bottle
    struct PillEntry {
        compound: String,
        title: String,
        content: String,
    }

    let mut pill_stmt = conn.prepare(
        "SELECT p.compound, p.title, p.content
         FROM pills p
         JOIN prescriptions rx ON p.prescription_id = rx.id
         WHERE rx.bottle_id = ?1 AND p.deleted_at IS NULL
         ORDER BY p.created_at DESC
         LIMIT ?2",
    )?;

    let pills: Vec<PillEntry> = pill_stmt
        .query_map(params![bottle_id, pill_limit], |row| {
            Ok(PillEntry {
                compound: row.get(0)?,
                title: row.get(1)?,
                content: row.get(2)?,
            })
        })?
        .collect::<rusqlite::Result<Vec<_>>>()
        .context("error al cargar pills para contexto")?;

    // Formatear como Markdown
    let prescription_count = prescriptions.len();
    let pill_count = pills.len();
    let mut md = String::new();

    if !prescriptions.is_empty() {
        md.push_str("## Recent Prescriptions\n\n");
        for rx in &prescriptions {
            let status = if rx.ended_at.is_some() {
                "closed"
            } else {
                "open"
            };
            let date = rx.started_at.get(..10).unwrap_or(&rx.started_at);
            md.push_str(&format!(
                "- **{}** ({}, {}) [{} pills]\n",
                rx.title, date, status, rx.pill_count
            ));
        }
        md.push('\n');
    }

    if !pills.is_empty() {
        md.push_str("## Recent Pills\n\n");
        for pill in &pills {
            // Truncar el contenido a 400 chars para mantener el contexto manejable
            let snippet = if pill.content.len() > 400 {
                format!("{}…", &pill.content[..400])
            } else {
                pill.content.clone()
            };
            md.push_str(&format!(
                "- [{}] **{}**: {}\n",
                pill.compound, pill.title, snippet
            ));
        }
    }

    Ok(ContextResult {
        context: md,
        prescription_count,
        pill_count,
    })
}

fn row_to_search_result(row: &rusqlite::Row<'_>) -> rusqlite::Result<SearchResult> {
    Ok(SearchResult {
        id: row.get(0)?,
        sync_id: row.get(1)?,
        compound: row.get(2)?,
        title: row.get(3)?,
        snippet: row.get(4)?,
        updated_at: row.get(5)?,
        rank: row.get(6)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::connection::open_in_memory;
    use crate::db::store::{bottles, capsules, pills, prescriptions};
    use crate::domain::bottle::{BottleScope, NewBottle};
    use crate::domain::capsule::{CapsuleCompound, NewCapsule};
    use crate::domain::pill::{NewPill, PillCompound};
    use crate::domain::prescription::NewPrescription;
    use crate::domain::search::SearchParams;

    fn setup_with_pills(conn: &mut Connection) -> i64 {
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
                bottle_id: bottle.id,
                title: "Sesión de búsqueda".into(),
            },
        )
        .unwrap();

        for (title, content, compound) in [
            (
                "JWT tokens con refresh",
                "Implementamos stateless JWT con refresh tokens almacenados en SQLite.",
                PillCompound::Decision,
            ),
            (
                "Race condition en dedup",
                "BEGIN IMMEDIATE previene race conditions en escrituras concurrentes.",
                PillCompound::Bugfix,
            ),
            (
                "FTS5 tokenizer unicode61",
                "El tokenizer unicode61 normaliza acentos automáticamente.",
                PillCompound::Discovery,
            ),
        ] {
            pills::take(
                conn,
                &NewPill {
                    title: title.into(),
                    content: content.into(),
                    compound,
                    prescription_id: rx.id.clone(),
                    dispenser: None,
                    author_name: None,
                    author_email: None,
                },
            )
            .unwrap();
        }

        bottle.id
    }

    #[test]
    fn pill_find_returns_results() {
        let mut conn = open_in_memory().unwrap();
        setup_with_pills(&mut conn);

        let results = pill_find(
            &conn,
            &SearchParams {
                query: "JWT".into(),
                bottle_id: None,
                compound: None,
                limit: Some(10),
            },
        )
        .unwrap();

        assert!(!results.is_empty());
        assert!(results[0].title.contains("JWT"));
    }

    #[test]
    fn pill_find_filters_by_compound() {
        let mut conn = open_in_memory().unwrap();
        setup_with_pills(&mut conn);

        let results = pill_find(
            &conn,
            &SearchParams {
                query: "BEGIN".into(),
                bottle_id: None,
                compound: Some("bugfix".into()),
                limit: None,
            },
        )
        .unwrap();

        assert!(!results.is_empty());
        assert_eq!(results[0].compound, "bugfix");
    }

    #[test]
    fn pill_find_empty_query_returns_empty() {
        let conn = open_in_memory().unwrap();
        let results = pill_find(
            &conn,
            &SearchParams {
                query: "".into(),
                bottle_id: None,
                compound: None,
                limit: None,
            },
        )
        .unwrap();
        assert!(results.is_empty());
    }

    #[test]
    fn capsule_find_works() {
        let mut conn = open_in_memory().unwrap();

        capsules::take(
            &mut conn,
            &NewCapsule {
                title: "Snake_case en todos los proyectos".into(),
                content: "Prefiero snake_case incluso en TypeScript.".into(),
                compound: CapsuleCompound::Convention,
            },
        )
        .unwrap();

        let results = capsule_find(&conn, "snake_case", None, None).unwrap();
        assert!(!results.is_empty());
        assert!(results[0].title.contains("Snake_case"));
    }

    #[test]
    fn pill_context_formats_markdown() {
        let mut conn = open_in_memory().unwrap();
        let bottle_id = setup_with_pills(&mut conn);

        let ctx = pill_context(&conn, bottle_id, 5, 30).unwrap();
        assert!(ctx.prescription_count > 0);
        assert!(ctx.pill_count > 0);
        assert!(ctx.context.contains("## Recent Prescriptions"));
        assert!(ctx.context.contains("## Recent Pills"));
        assert!(ctx.context.contains("[decision]") || ctx.context.contains("[bugfix]"));
    }

    #[test]
    fn sanitize_fts_query_wraps_terms() {
        assert_eq!(
            sanitize_fts_query("fix auth bug"),
            "\"fix\" \"auth\" \"bug\""
        );
        assert_eq!(sanitize_fts_query("AND OR NOT"), "\"AND\" \"OR\" \"NOT\"");
        assert_eq!(sanitize_fts_query(""), "");
    }
}
