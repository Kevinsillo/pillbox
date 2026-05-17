//! Helper genérico para incrementar el contador `views` de una entidad.
//!
//! Diseñado para ser llamado desde los handlers de lectura (MCP/WebUI/CLI)
//! después de una lectura exitosa. Devuelve el nuevo valor del contador para
//! que el caller pueda incluirlo en la respuesta sin necesidad de una segunda
//! query.
//!
//! El parámetro `table` se valida contra una whitelist estática — nunca se
//! interpola un valor controlado por el usuario directamente en SQL. El `id`
//! viaja como parámetro vinculado.

use anyhow::{Context, Result};
use rusqlite::{params, Connection};

/// Incrementa atómicamente `views` en la tabla indicada y devuelve el nuevo valor.
///
/// `table` debe ser uno de: `bottles`, `prescriptions`, `pills`, `capsules`.
/// Cualquier otro valor produce un error (`invalid_table`).
///
/// Si la fila no existe (id desconocido), devuelve un error (`not_found`).
///
/// La query usa `RETURNING views`, soportado por SQLite ≥ 3.35.
pub fn increment_views(conn: &mut Connection, table: &str, id: &str) -> Result<i64> {
    // Whitelist explícita: el nombre de tabla se interpola en el SQL, así que
    // debe ser un literal conocido. NUNCA aceptar input externo aquí.
    let table = match table {
        "bottles" => "bottles",
        "prescriptions" => "prescriptions",
        "pills" => "pills",
        "capsules" => "capsules",
        other => {
            anyhow::bail!("invalid_table: {other} (expected bottles|prescriptions|pills|capsules)")
        }
    };

    let sql = format!("UPDATE {table} SET views = views + 1 WHERE id = ?1 RETURNING views");

    match conn.query_row(&sql, params![id], |row| row.get::<_, i64>(0)) {
        Ok(views) => Ok(views),
        Err(rusqlite::Error::QueryReturnedNoRows) => {
            anyhow::bail!("not_found: no row in {table} with id={id}")
        }
        Err(e) => Err(e).with_context(|| format!("failed to increment views on {table}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::connection::open_in_memory;
    use crate::db::store::{bottles, capsules, pills, prescriptions};
    use crate::db::DbScope;
    use crate::domain::bottle::{BottleScope, NewBottle};
    use crate::domain::capsule::NewCapsule;
    use crate::domain::pill::NewPill;
    use crate::domain::prescription::NewPrescription;

    fn setup(conn: &mut Connection) -> (String, String, String, String) {
        let bottle = bottles::create(
            conn,
            &NewBottle {
                name: "ctr-test".into(),
                display_name: "Ctr".into(),
                directory: "/tmp/ctr".into(),
                scope: BottleScope::Local,
            },
        )
        .unwrap();
        let rx = prescriptions::open(
            conn,
            &NewPrescription {
                bottle_id: bottle.id.clone(),
                title: "rx".into(),
                author_name: None,
                author_email: None,
            },
        )
        .unwrap();
        let pill = pills::take(
            conn,
            &NewPill {
                title: "p".into(),
                content: "c".into(),
                compound: "decision".into(),
                prescription_id: rx.id.clone(),
                author_name: None,
                author_email: None,
            },
        )
        .unwrap();
        let cap = capsules::take(
            conn,
            &NewCapsule {
                title: "cap".into(),
                content: "c".into(),
                compound: "convention".into(),
            },
        )
        .unwrap();
        (bottle.id, rx.id, pill.id, cap.id)
    }

    #[test]
    fn increment_views_on_each_entity() {
        let mut conn = open_in_memory(DbScope::Global).unwrap();
        let (bottle_id, rx_id, pill_id, cap_id) = setup(&mut conn);

        assert_eq!(
            increment_views(&mut conn, "bottles", &bottle_id).unwrap(),
            1
        );
        assert_eq!(
            increment_views(&mut conn, "bottles", &bottle_id).unwrap(),
            2
        );
        assert_eq!(
            increment_views(&mut conn, "prescriptions", &rx_id).unwrap(),
            1
        );
        assert_eq!(increment_views(&mut conn, "pills", &pill_id).unwrap(), 1);
        assert_eq!(increment_views(&mut conn, "capsules", &cap_id).unwrap(), 1);
    }

    #[test]
    fn increment_views_invalid_table_fails() {
        let mut conn = open_in_memory(DbScope::Global).unwrap();
        let err = increment_views(&mut conn, "users", "any-id").unwrap_err();
        assert!(err.to_string().contains("invalid_table"));
    }

    #[test]
    fn increment_views_missing_id_fails() {
        let mut conn = open_in_memory(DbScope::Global).unwrap();
        let err = increment_views(&mut conn, "pills", "00000000-0000-0000-0000-000000000000")
            .unwrap_err();
        assert!(err.to_string().contains("not_found"));
    }
}
