//! Integration tests para aislamiento de schema entre `DbScope::Local` y `DbScope::Global`.
//!
//! Verifica que:
//! - Local NO contiene tablas/triggers exclusivos de Global (capsules, capsules_fts,
//!   registered_bottles, capsules_* triggers).
//! - Global contiene todo lo de Local más lo suyo propio.
//! - Los triggers de FTS5 funcionan en cada scope.

use std::collections::HashSet;

use rusqlite::Connection;

use pillbox::db::{connection, DbScope};

// ─── Helpers ─────────────────────────────────────────────────────────────────

/// Devuelve el conjunto de nombres de tablas (incluye virtual + shadow tables de FTS5).
fn tables(conn: &Connection) -> HashSet<String> {
    let mut stmt = conn
        .prepare("SELECT name FROM sqlite_master WHERE type='table'")
        .unwrap();
    let rows = stmt
        .query_map([], |r| r.get::<_, String>(0))
        .unwrap()
        .map(|r| r.unwrap());
    rows.collect()
}

/// Devuelve el conjunto de nombres de triggers cuyo nombre case con `LIKE pattern`.
fn triggers_like(conn: &Connection, pattern: &str) -> HashSet<String> {
    let mut stmt = conn
        .prepare("SELECT name FROM sqlite_master WHERE type='trigger' AND name LIKE ?1")
        .unwrap();
    let rows = stmt
        .query_map([pattern], |r| r.get::<_, String>(0))
        .unwrap()
        .map(|r| r.unwrap());
    rows.collect()
}

/// Inserta un bottle + prescription mínimos y devuelve el id de la prescription.
/// Usa SQL directo — esto es un test de bajo nivel del schema, no de los stores.
fn seed_bottle_and_prescription(conn: &Connection) -> String {
    conn.execute(
        "INSERT INTO bottles (id, name, display_name, directory) \
         VALUES ('b-1', 'test-bottle', 'Test Bottle', '/tmp/test-bottle')",
        [],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO prescriptions (id, bottle_id, title) \
         VALUES ('p-1', 'b-1', 'Test Prescription')",
        [],
    )
    .unwrap();
    "p-1".to_string()
}

// ─── 4.1 — Local: tablas esperadas exactas ───────────────────────────────────

#[test]
fn local_schema_has_only_local_tables() {
    let conn = connection::open_in_memory(DbScope::Local).unwrap();
    let t = tables(&conn);

    // Tablas de dominio + auxiliares FTS5 + migraciones.
    let expected: HashSet<String> = [
        "bottles",
        "prescriptions",
        "pills",
        "pills_fts",
        "pills_fts_data",
        "pills_fts_idx",
        "pills_fts_docsize",
        "pills_fts_config",
        "schema_migrations",
    ]
    .iter()
    .map(|s| s.to_string())
    .collect();

    assert_eq!(
        t, expected,
        "Local scope debe contener EXACTAMENTE estas tablas. \
         Encontradas: {:?}. Esperadas: {:?}",
        t, expected
    );

    // Sanity: nada de Global presente.
    for forbidden in ["capsules", "capsules_fts", "registered_bottles"] {
        assert!(
            !t.contains(forbidden),
            "Local scope NO debe contener {}",
            forbidden
        );
    }
}

// ─── 4.2 — Global: incluye todo lo de Local + sus propias tablas ─────────────

#[test]
fn global_schema_has_all_tables() {
    let conn = connection::open_in_memory(DbScope::Global).unwrap();
    let t = tables(&conn);

    let must_exist = [
        // Locales
        "bottles",
        "prescriptions",
        "pills",
        "pills_fts",
        "schema_migrations",
        // Exclusivas de global
        "capsules",
        "capsules_fts",
        "registered_bottles",
    ];

    for name in must_exist {
        assert!(
            t.contains(name),
            "Global scope debe contener la tabla {}. Encontradas: {:?}",
            name,
            t
        );
    }
}

// ─── 4.3 — Local rechaza INSERT en capsules (tabla inexistente) ──────────────

#[test]
fn local_rejects_capsule_insert() {
    let conn = connection::open_in_memory(DbScope::Local).unwrap();

    let err = conn
        .execute(
            "INSERT INTO capsules (id, compound, title, content) \
             VALUES ('c-1', 'note', 't', 'c')",
            [],
        )
        .unwrap_err();

    let msg = err.to_string();
    assert!(
        msg.contains("no such table") && msg.contains("capsules"),
        "Esperado error 'no such table: capsules', got: {}",
        msg
    );
}

// ─── 4.4 — pills_fts triggers funcionan en ambos scopes ──────────────────────

fn assert_pills_fts_trigger_fires(scope: DbScope) {
    let conn = connection::open_in_memory(scope).unwrap();
    let prescription_id = seed_bottle_and_prescription(&conn);

    conn.execute(
        "INSERT INTO pills (id, compound, title, content, prescription_id) \
         VALUES ('pill-1', 'decision', 'Titulo', 'Contenido', ?1)",
        [&prescription_id],
    )
    .unwrap();

    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM pills_fts", [], |r| r.get(0))
        .unwrap();

    assert_eq!(
        count, 1,
        "El trigger pills_ai debería haber poblado pills_fts en scope {:?}",
        scope
    );

    // Verifica que el contenido es indexable vía MATCH.
    let hit: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM pills_fts WHERE pills_fts MATCH 'Titulo'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(hit, 1, "MATCH sobre pills_fts debería devolver la fila");
}

#[test]
fn pills_fts_trigger_fires_in_local() {
    assert_pills_fts_trigger_fires(DbScope::Local);
}

#[test]
fn pills_fts_trigger_fires_in_global() {
    assert_pills_fts_trigger_fires(DbScope::Global);
}

// ─── 4.5 — capsules_* triggers solo en Global ────────────────────────────────

#[test]
fn capsules_triggers_absent_in_local() {
    let conn = connection::open_in_memory(DbScope::Local).unwrap();
    let trigs = triggers_like(&conn, "capsules_%");
    assert!(
        trigs.is_empty(),
        "Local scope NO debe tener triggers capsules_*. Encontrados: {:?}",
        trigs
    );
}

#[test]
fn capsules_triggers_present_in_global() {
    let conn = connection::open_in_memory(DbScope::Global).unwrap();
    let trigs = triggers_like(&conn, "capsules_%");

    let expected: HashSet<String> = ["capsules_ai", "capsules_ad", "capsules_au"]
        .iter()
        .map(|s| s.to_string())
        .collect();

    assert_eq!(
        trigs, expected,
        "Global scope debe tener exactamente los 3 triggers capsules_*. \
         Encontrados: {:?}",
        trigs
    );
}
