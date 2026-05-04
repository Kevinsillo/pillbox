//! Operaciones de store para la entidad [`Prescription`].

use anyhow::{Context, Result};
use rusqlite::{params, Connection, TransactionBehavior};
use uuid::Uuid;

use crate::domain::prescription::{NewPrescription, Prescription};
use crate::error::PillboxError;

// ─── Store ────────────────────────────────────────────────────────────────────

/// Abre una nueva prescription para el bottle dado.
///
/// Falla con `PillboxError::PrescriptionAlreadyOpen` si ya hay una prescription activa
/// (no cerrada ni descartada) para ese bottle.
pub fn open(conn: &mut Connection, input: &NewPrescription) -> Result<Prescription> {
    let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;

    // Verificar si ya hay una prescription abierta para este bottle
    let existing = match tx.query_row(
        "SELECT rx.id, rx.title, rx.started_at,
                (SELECT COUNT(*) FROM pills p
                 WHERE p.prescription_id = rx.id AND p.deleted_at IS NULL) AS pill_count
         FROM prescriptions rx
         WHERE rx.bottle_id = ?1
           AND rx.ended_at IS NULL
           AND rx.deleted_at IS NULL",
        params![input.bottle_id],
        |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, i64>(3)?,
            ))
        },
    ) {
        Ok(row) => Some(row),
        Err(rusqlite::Error::QueryReturnedNoRows) => None,
        Err(e) => return Err(e).context("failed to check for open prescription"),
    };

    if let Some((id, title, started_at, pill_count)) = existing {
        return Err(PillboxError::PrescriptionAlreadyOpen {
            id,
            title,
            started_at,
            pill_count,
        }
        .into());
    }

    // Verificar que el bottle existe
    let bottle_exists: bool = tx.query_row(
        "SELECT EXISTS(SELECT 1 FROM bottles WHERE id = ?1)",
        params![input.bottle_id],
        |row| row.get(0),
    )?;

    if !bottle_exists {
        return Err(PillboxError::BottleNotFound {
            bottle_id: input.bottle_id.clone(),
        }
        .into());
    }

    let id = Uuid::now_v7().to_string();

    tx.execute(
        "INSERT INTO prescriptions (id, bottle_id, title, author_name, author_email)
         VALUES (?1, ?2, ?3, ?4, ?5)",
        params![
            id,
            input.bottle_id,
            input.title,
            input.author_name,
            input.author_email
        ],
    )
    .context("failed to insert prescription")?;

    let prescription = tx
        .query_row(
            "SELECT id, bottle_id, title, author_name, author_email, started_at, ended_at, deleted_at
         FROM prescriptions WHERE id = ?1",
            params![id],
            row_to_prescription,
        )
        .context("failed to read newly opened prescription")?;

    tx.commit()?;
    Ok(prescription)
}

/// Cierra una prescription existente (establece `ended_at`).
pub fn close(conn: &mut Connection, id: &str) -> Result<Prescription> {
    let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;

    let affected = tx
        .execute(
            "UPDATE prescriptions SET ended_at = datetime('now')
         WHERE id = ?1 AND ended_at IS NULL AND deleted_at IS NULL",
            params![id],
        )
        .context("failed to close prescription")?;

    if affected == 0 {
        return Err(PillboxError::PrescriptionNotFoundOrClosed { id: id.to_string() }.into());
    }

    let prescription = tx
        .query_row(
            "SELECT id, bottle_id, title, author_name, author_email, started_at, ended_at, deleted_at
         FROM prescriptions WHERE id = ?1",
            params![id],
            row_to_prescription,
        )
        .context("failed to read closed prescription")?;

    tx.commit()?;
    Ok(prescription)
}

/// Soft delete de una prescription y todas sus pills.
///
/// El cascade es lógico: establece `deleted_at` en la prescription y en
/// todas sus pills activas. No borra filas de forma permanente.
///
/// # Errors
///
/// Devuelve [`PillboxError::PrescriptionNotFound`] si la prescription no existe
/// o ya estaba descartada.
pub fn discard(conn: &mut Connection, id: &str) -> Result<()> {
    let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;

    let affected = tx.execute(
        "UPDATE prescriptions SET deleted_at = datetime('now')
         WHERE id = ?1 AND deleted_at IS NULL",
        params![id],
    )?;

    if affected == 0 {
        return Err(PillboxError::PrescriptionNotFound { id: id.to_string() }.into());
    }

    tx.execute(
        "UPDATE pills SET deleted_at = datetime('now')
         WHERE prescription_id = ?1 AND deleted_at IS NULL",
        params![id],
    )?;

    tx.commit()?;
    Ok(())
}

/// Hard delete de una prescription y todos sus datos relacionados (irreversible).
///
/// Elimina en orden dentro de una tx IMMEDIATE:
/// pills → prescriptions.
///
/// Falla con `PillboxError::PrescriptionNotFound` si la prescription no existe.
pub fn hard_delete(conn: &mut Connection, id: &str) -> Result<()> {
    let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;

    // Verificar que la prescription existe
    let exists: bool = tx.query_row(
        "SELECT EXISTS(SELECT 1 FROM prescriptions WHERE id = ?1)",
        params![id],
        |row| row.get(0),
    )?;

    if !exists {
        return Err(PillboxError::PrescriptionNotFound { id: id.to_string() }.into());
    }

    // 1. Eliminar pills de la prescription
    tx.execute(
        "DELETE FROM pills WHERE prescription_id = ?1",
        params![id],
    )
    .context("failed to delete pills for prescription")?;

    // 2. Eliminar la prescription
    tx.execute("DELETE FROM prescriptions WHERE id = ?1", params![id])
        .context("failed to delete prescription")?;

    tx.commit()?;
    Ok(())
}

/// Lee una prescription por ID exacto o prefijo de UUID incluyendo descartadas.
pub fn read_any(conn: &Connection, id: &str) -> Result<Option<Prescription>> {
    let pattern = format!("{}%", id);
    match conn.query_row(
        "SELECT id, bottle_id, title, author_name, author_email, started_at, ended_at, deleted_at
         FROM prescriptions
         WHERE (id = ?1 OR id LIKE ?2)
         ORDER BY started_at DESC LIMIT 1",
        params![id, pattern],
        row_to_prescription,
    ) {
        Ok(rx) => Ok(Some(rx)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(e).context("failed to read prescription"),
    }
}

/// Lee una prescription por ID exacto o prefijo de UUID (no descartada).
pub fn read(conn: &Connection, id: &str) -> Result<Option<Prescription>> {
    let pattern = format!("{}%", id);
    match conn.query_row(
        "SELECT id, bottle_id, title, author_name, author_email, started_at, ended_at, deleted_at
         FROM prescriptions
         WHERE (id = ?1 OR id LIKE ?2) AND deleted_at IS NULL
         ORDER BY started_at DESC LIMIT 1",
        params![id, pattern],
        row_to_prescription,
    ) {
        Ok(rx) => Ok(Some(rx)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(e).context("failed to read prescription"),
    }
}

/// Devuelve las últimas N prescriptions de un bottle (más recientes primero).
pub fn count_by_bottle(conn: &Connection, bottle_id: &str) -> Result<u32> {
    let n: u32 = conn.query_row(
        "SELECT COUNT(*) FROM prescriptions WHERE bottle_id = ?1",
        params![bottle_id],
        |r| r.get(0),
    )?;
    Ok(n)
}

pub fn list_by_bottle(conn: &Connection, bottle_id: &str, limit: u32) -> Result<Vec<Prescription>> {
    let mut stmt = conn.prepare(
        "SELECT id, bottle_id, title, author_name, author_email, started_at, ended_at, deleted_at
         FROM prescriptions
         WHERE bottle_id = ?1
         ORDER BY started_at DESC
         LIMIT ?2",
    )?;

    let rows = stmt
        .query_map(params![bottle_id, limit], row_to_prescription)?
        .collect::<rusqlite::Result<Vec<_>>>()
        .context("failed to list prescriptions")?;

    Ok(rows)
}

/// Mapea una fila de SQLite al tipo [`Prescription`].
fn row_to_prescription(row: &rusqlite::Row<'_>) -> rusqlite::Result<Prescription> {
    Ok(Prescription {
        id: row.get(0)?,
        bottle_id: row.get(1)?,
        title: row.get(2)?,
        author_name: row.get(3)?,
        author_email: row.get(4)?,
        started_at: row.get(5)?,
        ended_at: row.get(6)?,
        deleted_at: row.get(7)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::connection::open_in_memory;
    use crate::db::store::bottles;
    use crate::domain::bottle::{BottleScope, NewBottle};
    use crate::error::PillboxError;

    fn make_bottle(conn: &mut Connection, name: &str) -> String {
        bottles::create(
            conn,
            &NewBottle {
                name: name.into(),
                display_name: name.into(),
                directory: format!("/tmp/{}", name),
                scope: BottleScope::Local,
            },
        )
        .unwrap()
        .id
    }

    #[test]
    fn open_and_close() {
        let mut conn = open_in_memory().unwrap();
        let bottle_id = make_bottle(&mut conn, "test");

        let rx = open(
            &mut conn,
            &NewPrescription {
                bottle_id: bottle_id.clone(),
                title: "Implementar auth".into(),
                author_name: None,
                author_email: None,
            },
        )
        .unwrap();

        assert_eq!(rx.bottle_id, bottle_id);
        assert!(rx.ended_at.is_none());

        let closed = close(&mut conn, &rx.id).unwrap();
        assert!(closed.ended_at.is_some());
    }

    #[test]
    fn collision_returns_typed_error() {
        let mut conn = open_in_memory().unwrap();
        let bottle_id = make_bottle(&mut conn, "colision");

        open(
            &mut conn,
            &NewPrescription {
                bottle_id: bottle_id.clone(),
                title: "Primera sesión".into(),
                author_name: None,
                author_email: None,
            },
        )
        .unwrap();

        let err = open(
            &mut conn,
            &NewPrescription {
                bottle_id: bottle_id.clone(),
                title: "Segunda sesión".into(),
                author_name: None,
                author_email: None,
            },
        )
        .unwrap_err();

        let already_open = err.downcast_ref::<PillboxError>();
        assert!(matches!(
            already_open,
            Some(PillboxError::PrescriptionAlreadyOpen { .. })
        ));
        if let Some(PillboxError::PrescriptionAlreadyOpen {
            title, pill_count, ..
        }) = already_open
        {
            assert_eq!(title, "Primera sesión");
            assert_eq!(*pill_count, 0);
        }
    }

    #[test]
    fn bottle_not_found_fails() {
        let mut conn = open_in_memory().unwrap();
        let err = open(
            &mut conn,
            &NewPrescription {
                bottle_id: "uuid-inexistente".into(),
                title: "Sesión".into(),
                author_name: None,
                author_email: None,
            },
        )
        .unwrap_err();
        assert!(err.to_string().contains("bottle_not_found"));
    }

    #[test]
    fn close_nonexistent_fails() {
        let mut conn = open_in_memory().unwrap();
        assert!(close(&mut conn, "id-inexistente").is_err());
    }

    #[test]
    fn read_existing_prescription() {
        let mut conn = open_in_memory().unwrap();
        let bottle_id = make_bottle(&mut conn, "read-test");
        let rx = open(
            &mut conn,
            &NewPrescription {
                bottle_id,
                title: "Lectura".into(),
                author_name: None,
                author_email: None,
            },
        )
        .unwrap();

        let found = read(&conn, &rx.id).unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().title, "Lectura");
    }

    #[test]
    fn read_discarded_returns_none() {
        let mut conn = open_in_memory().unwrap();
        let bottle_id = make_bottle(&mut conn, "read-discard");
        let rx = open(
            &mut conn,
            &NewPrescription {
                bottle_id,
                title: "A descartar".into(),
                author_name: None,
                author_email: None,
            },
        )
        .unwrap();

        discard(&mut conn, &rx.id).unwrap();
        assert!(read(&conn, &rx.id).unwrap().is_none());
    }

    #[test]
    fn list_by_bottle_returns_multiple() {
        let mut conn = open_in_memory().unwrap();
        let bottle_id = make_bottle(&mut conn, "list-test");

        let rx1 = open(
            &mut conn,
            &NewPrescription {
                bottle_id: bottle_id.clone(),
                title: "Sesión 1".into(),
                author_name: None,
                author_email: None,
            },
        )
        .unwrap();
        close(&mut conn, &rx1.id).unwrap();

        open(
            &mut conn,
            &NewPrescription {
                bottle_id: bottle_id.clone(),
                title: "Sesión 2".into(),
                author_name: None,
                author_email: None,
            },
        )
        .unwrap();

        let all = list_by_bottle(&conn, &bottle_id, 10).unwrap();
        assert_eq!(all.len(), 2);
        let titles: Vec<&str> = all.iter().map(|r| r.title.as_str()).collect();
        assert!(titles.contains(&"Sesión 1"));
        assert!(titles.contains(&"Sesión 2"));
    }

    #[test]
    fn list_by_bottle_respects_limit() {
        let mut conn = open_in_memory().unwrap();
        let bottle_id = make_bottle(&mut conn, "list-limit");

        for i in 0..5 {
            let rx = open(
                &mut conn,
                &NewPrescription {
                    bottle_id: bottle_id.clone(),
                    title: format!("S{i}"),
                    author_name: None,
                    author_email: None,
                },
            )
            .unwrap();
            close(&mut conn, &rx.id).unwrap();
        }

        let limited = list_by_bottle(&conn, &bottle_id, 3).unwrap();
        assert_eq!(limited.len(), 3);
    }

    #[test]
    fn discard_twice_fails() {
        let mut conn = open_in_memory().unwrap();
        let bottle_id = make_bottle(&mut conn, "discard-twice");
        let rx = open(
            &mut conn,
            &NewPrescription {
                bottle_id,
                title: "S".into(),
                author_name: None,
                author_email: None,
            },
        )
        .unwrap();

        discard(&mut conn, &rx.id).unwrap();
        assert!(discard(&mut conn, &rx.id).is_err());
    }

    #[test]
    fn hard_delete_prescription_removes_all_rows() {
        let mut conn = open_in_memory().unwrap();
        let bottle_id = make_bottle(&mut conn, "hard-delete");

        let rx = open(
            &mut conn,
            &NewPrescription {
                bottle_id,
                title: "Sesión a purgar".into(),
                author_name: None,
                author_email: None,
            },
        )
        .unwrap();

        // Insertar una pill con sync_id único
        conn.execute(
            "INSERT INTO pills (sync_id, compound, title, content, prescription_id)
             VALUES ('sync-hd-001', 'decision', 'T', 'C', ?1)",
            params![rx.id],
        )
        .unwrap();

        hard_delete(&mut conn, &rx.id).unwrap();

        let rx_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM prescriptions WHERE id = ?1", params![rx.id], |r| r.get(0))
            .unwrap();
        let pill_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM pills WHERE prescription_id = ?1", params![rx.id], |r| r.get(0))
            .unwrap();

        assert_eq!(rx_count, 0);
        assert_eq!(pill_count, 0);
    }

    #[test]
    fn hard_delete_nonexistent_prescription_returns_error() {
        let mut conn = open_in_memory().unwrap();
        let err = hard_delete(&mut conn, "id-inexistente").unwrap_err();
        let typed = err.downcast_ref::<PillboxError>();
        assert!(matches!(typed, Some(PillboxError::PrescriptionNotFound { .. })));
    }

    #[test]
    fn discard_cascades_to_pills() {
        let mut conn = open_in_memory().unwrap();
        let bottle_id = make_bottle(&mut conn, "cascade");

        let rx = open(
            &mut conn,
            &NewPrescription {
                bottle_id,
                title: "Sesión a descartar".into(),
                author_name: None,
                author_email: None,
            },
        )
        .unwrap();

        // Insertar una pill directamente para verificar el cascade
        conn.execute(
            "INSERT INTO pills (sync_id, compound, title, content, prescription_id)
             VALUES ('sync-001', 'decision', 'T', 'C', ?1)",
            params![rx.id],
        )
        .unwrap();

        discard(&mut conn, &rx.id).unwrap();

        let pill_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM pills WHERE prescription_id = ?1 AND deleted_at IS NULL",
                params![rx.id],
                |r| r.get(0),
            )
            .unwrap();

        assert_eq!(pill_count, 0);
    }

    #[test]
    fn list_by_bottle_includes_archived() {
        let mut conn = open_in_memory().unwrap();
        let bottle_id = make_bottle(&mut conn, "archived-include");

        let rx = open(
            &mut conn,
            &NewPrescription {
                bottle_id: bottle_id.clone(),
                title: "Sesión a archivar".into(),
                author_name: None,
                author_email: None,
            },
        )
        .unwrap();
        discard(&mut conn, &rx.id).unwrap();

        let all = list_by_bottle(&conn, &bottle_id, 10).unwrap();
        assert_eq!(all.len(), 1);
        assert!(all[0].deleted_at.is_some());
    }

    #[test]
    fn list_by_bottle_active_and_archived_together() {
        let mut conn = open_in_memory().unwrap();
        let bottle_id = make_bottle(&mut conn, "active-and-archived");

        // Active prescription
        let rx_active = open(
            &mut conn,
            &NewPrescription {
                bottle_id: bottle_id.clone(),
                title: "Activa".into(),
                author_name: None,
                author_email: None,
            },
        )
        .unwrap();
        close(&mut conn, &rx_active.id).unwrap();

        // Archived prescription (re-open then discard)
        let rx_to_archive = open(
            &mut conn,
            &NewPrescription {
                bottle_id: bottle_id.clone(),
                title: "Archivada".into(),
                author_name: None,
                author_email: None,
            },
        )
        .unwrap();
        discard(&mut conn, &rx_to_archive.id).unwrap();

        let all = list_by_bottle(&conn, &bottle_id, 10).unwrap();
        assert_eq!(all.len(), 2);

        let active_count = all.iter().filter(|rx| rx.deleted_at.is_none()).count();
        let archived_count = all.iter().filter(|rx| rx.deleted_at.is_some()).count();
        assert_eq!(active_count, 1);
        assert_eq!(archived_count, 1);
    }

    #[test]
    fn read_any_returns_archived_prescription() {
        let mut conn = open_in_memory().unwrap();
        let bottle_id = make_bottle(&mut conn, "read-any-archived");
        let rx = open(
            &mut conn,
            &NewPrescription {
                bottle_id,
                title: "A archivar".into(),
                author_name: None,
                author_email: None,
            },
        )
        .unwrap();
        discard(&mut conn, &rx.id).unwrap();

        let found = read_any(&conn, &rx.id).unwrap();
        assert!(found.is_some());
        assert!(found.unwrap().deleted_at.is_some());
    }

    #[test]
    fn read_excludes_archived_but_read_any_does_not() {
        let mut conn = open_in_memory().unwrap();
        let bottle_id = make_bottle(&mut conn, "read-vs-read-any");
        let rx = open(
            &mut conn,
            &NewPrescription {
                bottle_id,
                title: "Test".into(),
                author_name: None,
                author_email: None,
            },
        )
        .unwrap();
        discard(&mut conn, &rx.id).unwrap();

        assert!(read(&conn, &rx.id).unwrap().is_none());
        assert!(read_any(&conn, &rx.id).unwrap().is_some());
    }

    #[test]
    fn author_fields_round_trip() {
        let mut conn = open_in_memory().unwrap();
        let bottle_id = make_bottle(&mut conn, "author-round-trip");

        let rx = open(
            &mut conn,
            &NewPrescription {
                bottle_id,
                title: "Con autor".into(),
                author_name: Some("Kevin Illanas".into()),
                author_email: Some("kevin@example.com".into()),
            },
        )
        .unwrap();

        assert_eq!(rx.author_name.as_deref(), Some("Kevin Illanas"));
        assert_eq!(rx.author_email.as_deref(), Some("kevin@example.com"));

        let found = read(&conn, &rx.id).unwrap().unwrap();
        assert_eq!(found.author_name.as_deref(), Some("Kevin Illanas"));
        assert_eq!(found.author_email.as_deref(), Some("kevin@example.com"));

        let listed = list_by_bottle(&conn, &rx.bottle_id, 10).unwrap();
        assert_eq!(listed[0].author_name.as_deref(), Some("Kevin Illanas"));
        assert_eq!(listed[0].author_email.as_deref(), Some("kevin@example.com"));
    }
}
