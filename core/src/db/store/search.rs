use std::collections::HashMap;

use anyhow::{Context, Result};
use rayon::prelude::*;
use rusqlite::{params, Connection};

use crate::domain::pill::Pill;
use crate::domain::search::{SearchParams, SearchResult};

// ─── Constantes fuzzy ────────────────────────────────────────────────────────

const FUZZY_MIN_LEN: usize = 4;
const FUZZY_THRESHOLD_SHORT: f64 = 0.85; // 4–6 chars
const FUZZY_THRESHOLD_LONG: f64 = 0.80; // 7+ chars
const FUZZY_MAX_LEN_DIFF: usize = 2;

// ─── Pipeline de query FTS5 + fuzzy ─────────────────────────────────────────

/// Extrae los términos individuales de la query del usuario, limpiando comillas.
fn extract_terms(query: &str) -> Vec<String> {
    query
        .split_whitespace()
        .map(|t| t.replace('"', ""))
        .filter(|t| !t.is_empty())
        .collect()
}

/// Obtiene todos los términos únicos del índice FTS5 indicado.
///
/// Usa una tabla virtual temporal `fts5vocab` que existe solo durante la
/// vida de la conexión — no requiere migración.
fn fetch_vocab(conn: &Connection, fts_table: &str) -> Result<Vec<String>> {
    let temp_name = format!("temp.{}_vocab", fts_table);
    conn.execute_batch(&format!(
        "CREATE VIRTUAL TABLE IF NOT EXISTS {temp_name}
         USING fts5vocab('main', '{fts_table}', 'row')"
    ))?;

    let mut stmt = conn.prepare(&format!("SELECT term FROM {temp_name}"))?;
    let vocab = stmt
        .query_map([], |row| row.get::<_, String>(0))?
        .collect::<rusqlite::Result<Vec<_>>>()
        .context("failed to read FTS5 vocabulary")?;

    Ok(vocab)
}

/// Para cada término de la query, encuentra términos del vocab con alta
/// similitud (Jaro-Winkler) usando rayon para paralelizar el escaneo.
///
/// Términos cortos (< FUZZY_MIN_LEN) se omiten para evitar falsos positivos.
/// Pre-filtra por diferencia de longitud antes de calcular similitud (O(1)).
fn fuzzy_expand<'a>(terms: &'a [String], vocab: &[String]) -> HashMap<&'a str, Vec<String>> {
    terms
        .iter()
        .map(|term| {
            let term_str = term.as_str();
            if term.len() < FUZZY_MIN_LEN {
                return (term_str, vec![]);
            }

            let threshold = if term.len() <= 6 {
                FUZZY_THRESHOLD_SHORT
            } else {
                FUZZY_THRESHOLD_LONG
            };

            let matches: Vec<String> = vocab
                .par_iter()
                .filter(|v| {
                    // Excluir el término exacto (ya lo cubre el prefix match)
                    v.as_str() != term_str
                    // Pre-filtro de longitud O(1): elimina ~80% de candidatos
                    && v.len().abs_diff(term.len()) <= FUZZY_MAX_LEN_DIFF
                    // Similitud Jaro-Winkler
                    && strsim::jaro_winkler(term_str, v.as_str()) >= threshold
                })
                .cloned()
                .collect();

            (term_str, matches)
        })
        .collect()
}

/// Construye la expresión FTS5 final combinando prefix search y expansión fuzzy.
///
/// Cada término genera un grupo OR: `("term"* OR "fuzzy1" OR "fuzzy2")`
/// Los grupos se unen con AND implícito (espacio).
///
/// Ejemplo: query "hexagnol auth" con fuzzy "hexagonal" →
/// `("hexagnol"* OR "hexagonal") AND "auth"*`
fn build_fts_query(terms: &[String], fuzzy_map: &HashMap<&str, Vec<String>>) -> String {
    terms
        .iter()
        .map(|term| {
            let prefix = format!("\"{}\"*", term);
            let fuzzy = fuzzy_map
                .get(term.as_str())
                .map(|v| v.as_slice())
                .unwrap_or(&[]);

            if fuzzy.is_empty() {
                prefix
            } else {
                let fuzzy_parts: String = fuzzy
                    .iter()
                    .map(|t| format!("\"{}\"", t))
                    .collect::<Vec<_>>()
                    .join(" OR ");
                format!("({prefix} OR {fuzzy_parts})")
            }
        })
        .collect::<Vec<_>>()
        .join(" AND ")
}

// ─── Sanitización simple (para queries sin fuzzy) ────────────────────────────

#[cfg(test)]
fn sanitize_fts_query(query: &str) -> String {
    let terms = extract_terms(query);
    if terms.is_empty() {
        return String::new();
    }
    terms
        .iter()
        .map(|t| format!("\"{}\"*", t))
        .collect::<Vec<_>>()
        .join(" ")
}

// ─── Pills ────────────────────────────────────────────────────────────────────

pub fn pill_find(conn: &Connection, params_in: &SearchParams) -> Result<Vec<SearchResult>> {
    let terms = extract_terms(&params_in.query);
    if terms.is_empty() {
        return Ok(vec![]);
    }

    let vocab = fetch_vocab(conn, "pills_fts")?;
    let fuzzy_map = fuzzy_expand(&terms, &vocab);
    let fts_query = build_fts_query(&terms, &fuzzy_map);

    let limit = params_in.limit.unwrap_or(20).min(100) as i64;

    let mut stmt = conn.prepare(
        "SELECT p.id,
                p.sync_id,
                p.compound,
                p.title,
                snippet(pills_fts, 1, '<b>', '</b>', '...', 12) AS snippet,
                p.created_at,
                p.updated_at,
                pills_fts.rank,
                p.prescription_id,
                rx.bottle_id
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
            |row| row_to_search_result(row, true),
        )?
        .collect::<rusqlite::Result<Vec<_>>>()
        .context("FTS5 pill search failed")?;

    Ok(results)
}

// ─── Capsules ─────────────────────────────────────────────────────────────────

pub fn capsule_find(
    conn: &Connection,
    query: &str,
    compound: Option<&str>,
    limit: Option<u32>,
) -> Result<Vec<SearchResult>> {
    let terms = extract_terms(query);
    if terms.is_empty() {
        return Ok(vec![]);
    }

    let vocab = fetch_vocab(conn, "capsules_fts")?;
    let fuzzy_map = fuzzy_expand(&terms, &vocab);
    let fts_query = build_fts_query(&terms, &fuzzy_map);

    let limit = limit.unwrap_or(20).min(100) as i64;

    let mut stmt = conn.prepare(
        "SELECT c.id,
                c.sync_id,
                c.compound,
                c.title,
                snippet(capsules_fts, 1, '<b>', '</b>', '...', 12) AS snippet,
                c.created_at,
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
        .query_map(params![fts_query, compound, limit], |row| {
            row_to_search_result(row, false)
        })?
        .collect::<rusqlite::Result<Vec<_>>>()
        .context("FTS5 capsule search failed")?;

    Ok(results)
}

// ─── Context ──────────────────────────────────────────────────────────────────

pub struct ContextResult {
    pub context: String,
    pub prescription_count: usize,
    pub pill_count: usize,
}

pub fn pill_context(
    conn: &Connection,
    bottle_id: &str,
    prescription_limit: u32,
    pill_limit: u32,
) -> Result<ContextResult> {
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
        .context("failed to load prescriptions for context")?;

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
        .context("failed to load pills for context")?;

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

pub fn recent_pills(conn: &Connection, bottle_id: &str, limit: u32) -> Result<Vec<Pill>> {
    let mut stmt = conn.prepare(
        "SELECT p.id, p.sync_id, p.compound, p.title, p.content, p.prescription_id,
                p.dispenser, p.author_name, p.author_email, p.created_at, p.updated_at, p.deleted_at
         FROM pills p
         JOIN prescriptions rx ON p.prescription_id = rx.id
         WHERE rx.bottle_id = ?1 AND p.deleted_at IS NULL
         ORDER BY p.created_at DESC
         LIMIT ?2",
    )?;

    let pills = stmt
        .query_map(params![bottle_id, limit], |row| {
            Ok(Pill {
                id: row.get(0)?,
                sync_id: row.get(1)?,
                compound: row.get(2)?,
                title: row.get(3)?,
                content: row.get(4)?,
                prescription_id: row.get(5)?,
                dispenser: row.get(6)?,
                author_name: row.get(7)?,
                author_email: row.get(8)?,
                created_at: row.get(9)?,
                updated_at: row.get(10)?,
                deleted_at: row.get(11)?,
            })
        })?
        .collect::<rusqlite::Result<Vec<_>>>()
        .context("failed to load recent pills")?;

    Ok(pills)
}

fn row_to_search_result(
    row: &rusqlite::Row<'_>,
    has_prescription_id: bool,
) -> rusqlite::Result<SearchResult> {
    Ok(SearchResult {
        id: row.get(0)?,
        sync_id: row.get(1)?,
        compound: row.get(2)?,
        title: row.get(3)?,
        snippet: row.get(4)?,
        created_at: row.get(5)?,
        updated_at: row.get(6)?,
        rank: row.get(7)?,
        prescription_id: if has_prescription_id {
            row.get(8)?
        } else {
            None
        },
        bottle_id: if has_prescription_id {
            row.get(9)?
        } else {
            None
        },
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
    fn pill_find_prefix_search() {
        let mut conn = open_in_memory().unwrap();
        setup_with_pills(&mut conn);

        // "tok" debe encontrar "tokens" y "tokenizer"
        let results = pill_find(
            &conn,
            &SearchParams {
                query: "tok".into(),
                bottle_id: None,
                compound: None,
                limit: Some(10),
            },
        )
        .unwrap();

        assert!(!results.is_empty());
    }

    #[test]
    fn pill_find_fuzzy_typo() {
        let mut conn = open_in_memory().unwrap();
        setup_with_pills(&mut conn);

        // "tokenir" (typo de "tokenizer") debe encontrar resultados via fuzzy
        let results = pill_find(
            &conn,
            &SearchParams {
                query: "tokenizr".into(),
                bottle_id: None,
                compound: None,
                limit: Some(10),
            },
        )
        .unwrap();

        assert!(
            !results.is_empty(),
            "fuzzy debe encontrar 'tokenizer' desde 'tokenizr'"
        );
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

        let ctx = pill_context(&conn, &bottle_id, 5, 30).unwrap();
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
            "\"fix\"* \"auth\"* \"bug\"*"
        );
        assert_eq!(
            sanitize_fts_query("AND OR NOT"),
            "\"AND\"* \"OR\"* \"NOT\"*"
        );
        assert_eq!(sanitize_fts_query(""), "");
    }

    #[test]
    fn build_fts_query_combines_prefix_and_fuzzy() {
        let terms = vec!["hexagnol".to_string(), "auth".to_string()];
        let mut fuzzy_map: HashMap<&str, Vec<String>> = HashMap::new();
        fuzzy_map.insert("hexagnol", vec!["hexagonal".to_string()]);
        fuzzy_map.insert("auth", vec![]);

        let query = build_fts_query(&terms, &fuzzy_map);
        assert_eq!(query, "(\"hexagnol\"* OR \"hexagonal\") AND \"auth\"*");
    }

    #[test]
    fn pill_find_mixed_fuzzy_and_bare_terms() {
        // Regresión: FTS5 no acepta `"bare"* (group)` con AND implícito.
        // Una query donde un término tiene expansión fuzzy y otro no debe
        // funcionar sin error (usamos AND explícito entre grupos).
        let mut conn = open_in_memory().unwrap();
        setup_with_pills(&mut conn);

        // "jwt" (3 chars, sin fuzzy) + "tokenizr" (typo, con fuzzy) = bare + group
        let results = pill_find(
            &conn,
            &SearchParams {
                query: "jwt tokenizr".into(),
                bottle_id: None,
                compound: None,
                limit: Some(10),
            },
        );

        // No debe producir error de sintaxis FTS5
        assert!(
            results.is_ok(),
            "no debe fallar con bare+group: {:?}",
            results.err()
        );
    }

    #[test]
    fn pill_find_multiple_fuzzy_terms() {
        // Regresión: dos términos con expansión fuzzy generan (group) AND (group),
        // que tampoco es válido con AND implícito en FTS5.
        let mut conn = open_in_memory().unwrap();
        setup_with_pills(&mut conn);

        // Ambos términos con typos → ambos generan grupos
        let results = pill_find(
            &conn,
            &SearchParams {
                query: "tokenizr concurente".into(),
                bottle_id: None,
                compound: None,
                limit: Some(10),
            },
        );

        assert!(
            results.is_ok(),
            "no debe fallar con group AND group: {:?}",
            results.err()
        );
    }

    #[test]
    fn pill_find_filters_by_bottle_id() {
        let mut conn = open_in_memory().unwrap();
        let bottle_a = setup_with_pills(&mut conn);

        // Segundo bottle con pills distintas
        let bottle_b = bottles::create(
            &mut conn,
            &NewBottle {
                name: "otro-bottle".into(),
                display_name: "Otro".into(),
                directory: "/tmp/otro".into(),
                scope: BottleScope::Local,
            },
        )
        .unwrap();

        let rx_b = prescriptions::open(
            &mut conn,
            &NewPrescription {
                bottle_id: bottle_b.id.clone(),
                title: "Sesión B".into(),
            },
        )
        .unwrap();

        pills::take(
            &mut conn,
            &NewPill {
                title: "JWT en bottle B".into(),
                content: "Otro contexto JWT completamente diferente.".into(),
                compound: PillCompound::Decision,
                prescription_id: rx_b.id.clone(),
                dispenser: None,
                author_name: None,
                author_email: None,
            },
        )
        .unwrap();

        // Buscar JWT solo en bottle_a
        let results_a = pill_find(
            &conn,
            &SearchParams {
                query: "JWT".into(),
                bottle_id: Some(bottle_a.clone()),
                compound: None,
                limit: Some(10),
            },
        )
        .unwrap();

        // Buscar JWT solo en bottle_b
        let results_b = pill_find(
            &conn,
            &SearchParams {
                query: "JWT".into(),
                bottle_id: Some(bottle_b.id.clone()),
                compound: None,
                limit: Some(10),
            },
        )
        .unwrap();

        assert!(!results_a.is_empty());
        assert!(!results_b.is_empty());
        // Cada búsqueda devuelve pills de su propio bottle
        for r in &results_a {
            assert_eq!(r.bottle_id, Some(bottle_a.clone()));
        }
        for r in &results_b {
            assert_eq!(r.bottle_id, Some(bottle_b.id.clone()));
        }
    }

    #[test]
    fn capsule_find_filters_by_compound() {
        let mut conn = open_in_memory().unwrap();

        capsules::take(
            &mut conn,
            &NewCapsule {
                title: "Convención de nombres".into(),
                content: "Usar snake_case en Rust.".into(),
                compound: CapsuleCompound::Convention,
            },
        )
        .unwrap();

        capsules::take(
            &mut conn,
            &NewCapsule {
                title: "Flujo de despliegue snake_case".into(),
                content: "Pipeline automatizado para snake_case deployments.".into(),
                compound: CapsuleCompound::Workflow,
            },
        )
        .unwrap();

        let conventions = capsule_find(&conn, "snake_case", Some("convention"), None).unwrap();
        assert_eq!(conventions.len(), 1);
        assert_eq!(conventions[0].compound, "convention");
    }

    #[test]
    fn capsule_find_empty_query_returns_empty() {
        let conn = open_in_memory().unwrap();
        let results = capsule_find(&conn, "", None, None).unwrap();
        assert!(results.is_empty());
    }

    #[test]
    fn pill_context_empty_bottle_returns_empty_context() {
        let mut conn = open_in_memory().unwrap();
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

        let ctx = pill_context(&conn, &bottle.id, 5, 30).unwrap();
        assert_eq!(ctx.prescription_count, 0);
        assert_eq!(ctx.pill_count, 0);
        assert!(ctx.context.is_empty());
    }

    #[test]
    fn recent_pills_returns_pills_for_bottle() {
        let mut conn = open_in_memory().unwrap();
        let bottle_id = setup_with_pills(&mut conn);

        let recent = recent_pills(&conn, &bottle_id, 10).unwrap();
        assert!(!recent.is_empty());
    }

    #[test]
    fn recent_pills_empty_for_unknown_bottle() {
        let conn = open_in_memory().unwrap();
        let recent = recent_pills(&conn, "uuid-inexistente", 10).unwrap();
        assert!(recent.is_empty());
    }
}
