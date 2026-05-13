//! Operaciones de store para la entidad [`Prescription`].

use anyhow::{Context, Result};
use rusqlite::{params, Connection, TransactionBehavior};
use uuid::Uuid;

use crate::db::store::id_resolver::resolve_id;
use crate::db::store::ListFilter;
use crate::domain::prescription::{NewPrescription, Prescription};
use crate::error::PillboxError;

// ─── Store ────────────────────────────────────────────────────────────────────

/// Abre una nueva prescription para el bottle dado.
///
/// Falla con `PillboxError::PrescriptionAlreadyOpen` si ya hay una prescription activa
/// (no cerrada ni descartada) para ese bottle.
pub fn open(conn: &mut Connection, input: &NewPrescription) -> Result<Prescription> {
    // Resolver el bottle_id (acepta UUID completo o prefijo ≥8 chars) y validar
    // que existe antes de abrir la prescription.
    let resolved_bottle_id = resolve_id(conn, "bottles", &input.bottle_id)?
        .ok_or_else(|| PillboxError::BottleNotFound {
            bottle_id: input.bottle_id.clone(),
        })?;

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
        params![resolved_bottle_id],
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

    let id = Uuid::now_v7().to_string();

    tx.execute(
        "INSERT INTO prescriptions (id, bottle_id, title, author_name, author_email)
         VALUES (?1, ?2, ?3, ?4, ?5)",
        params![
            id,
            resolved_bottle_id,
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
///
/// Acepta UUID completo o prefijo ≥8 chars.
pub fn close(conn: &mut Connection, id: &str) -> Result<Prescription> {
    let resolved_id = resolve_id(conn, "prescriptions", id)?
        .ok_or_else(|| PillboxError::PrescriptionNotFoundOrClosed { id: id.to_string() })?;

    let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;

    let affected = tx
        .execute(
            "UPDATE prescriptions SET ended_at = datetime('now')
         WHERE id = ?1 AND ended_at IS NULL AND deleted_at IS NULL",
            params![resolved_id],
        )
        .context("failed to close prescription")?;

    if affected == 0 {
        return Err(PillboxError::PrescriptionNotFoundOrClosed { id: id.to_string() }.into());
    }

    let prescription = tx
        .query_row(
            "SELECT id, bottle_id, title, author_name, author_email, started_at, ended_at, deleted_at
         FROM prescriptions WHERE id = ?1",
            params![resolved_id],
            row_to_prescription,
        )
        .context("failed to read closed prescription")?;

    tx.commit()?;
    Ok(prescription)
}

/// Reabre una prescription cerrada (limpia `ended_at`).
///
/// Acepta UUID completo o prefijo ≥8 chars.
///
/// # Errores
/// - [`PillboxError::PrescriptionNotFound`] si no existe (incluye prefijo no resoluble).
/// - [`PillboxError::PrescriptionNotFound`] si está descartada (`deleted_at IS NOT NULL`).
/// - [`PillboxError::PrescriptionAlreadyOpen`] si ya está abierta (`ended_at IS NULL`).
/// - [`PillboxError::PrescriptionAlreadyOpenInBottle`] si otra prescription del mismo
///   bottle está abierta (colisión).
pub fn reopen(conn: &mut Connection, id: &str) -> Result<Prescription> {
    let resolved_id = resolve_id(conn, "prescriptions", id)?
        .ok_or_else(|| PillboxError::PrescriptionNotFound { id: id.to_string() })?;

    // Pre-check fuera de tx: existencia + estado actual.
    let (ended_at, deleted_at, bottle_id): (Option<String>, Option<String>, String) = match conn
        .query_row(
            "SELECT ended_at, deleted_at, bottle_id FROM prescriptions WHERE id = ?1",
            params![resolved_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        ) {
        Ok(t) => t,
        Err(rusqlite::Error::QueryReturnedNoRows) => {
            return Err(PillboxError::PrescriptionNotFound { id: id.to_string() }.into());
        }
        Err(e) => return Err(e).context("failed to load prescription state for reopen"),
    };

    if deleted_at.is_some() {
        // Target descartada — reusamos PrescriptionNotFound (no se puede reabrir una papelera).
        return Err(PillboxError::PrescriptionNotFound { id: id.to_string() }.into());
    }

    if ended_at.is_none() {
        // Ya está abierta — devolvemos PrescriptionAlreadyOpen con los datos completos.
        let (title, started_at, pill_count) = conn.query_row(
            "SELECT title, started_at,
                    (SELECT COUNT(*) FROM pills p
                     WHERE p.prescription_id = ?1 AND p.deleted_at IS NULL) AS pill_count
             FROM prescriptions WHERE id = ?1",
            params![resolved_id],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, i64>(2)?,
                ))
            },
        )?;
        return Err(PillboxError::PrescriptionAlreadyOpen {
            id: resolved_id,
            title,
            started_at,
            pill_count,
        }
        .into());
    }

    // Tx IMMEDIATE: comprobación de colisión + UPDATE + lectura final.
    let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;

    let collision: Option<String> = match tx.query_row(
        "SELECT id FROM prescriptions
         WHERE bottle_id = ?1 AND ended_at IS NULL AND deleted_at IS NULL",
        params![bottle_id],
        |row| row.get::<_, String>(0),
    ) {
        Ok(existing) => Some(existing),
        Err(rusqlite::Error::QueryReturnedNoRows) => None,
        Err(e) => return Err(e).context("failed to check bottle for open prescription"),
    };

    if let Some(existing_id) = collision {
        return Err(PillboxError::PrescriptionAlreadyOpenInBottle {
            bottle_id,
            existing_id,
        }
        .into());
    }

    let affected = tx
        .execute(
            "UPDATE prescriptions SET ended_at = NULL
             WHERE id = ?1 AND ended_at IS NOT NULL AND deleted_at IS NULL",
            params![resolved_id],
        )
        .context("failed to reopen prescription")?;

    if affected == 0 {
        // Race: el estado cambió entre el pre-check y el UPDATE.
        return Err(PillboxError::PrescriptionNotFound { id: id.to_string() }.into());
    }

    let prescription = tx
        .query_row(
            "SELECT id, bottle_id, title, author_name, author_email, started_at, ended_at, deleted_at
             FROM prescriptions WHERE id = ?1",
            params![resolved_id],
            row_to_prescription,
        )
        .context("failed to read reopened prescription")?;

    tx.commit()?;
    Ok(prescription)
}

/// Verifica que una prescription está abierta (no cerrada ni descartada).
///
/// Devuelve `Err(PrescriptionClosed)` si `ended_at IS NOT NULL` o
/// `deleted_at IS NOT NULL`. Propaga `PrescriptionNotFound` si no existe.
///
/// Usado por las operaciones de edición de pills (`revise`, `discard`) como
/// guard previo a modificar el contenido.
pub(crate) fn ensure_rx_open(conn: &Connection, rx_id: &str) -> Result<()> {
    let (ended_at, deleted_at): (Option<String>, Option<String>) = match conn.query_row(
        "SELECT ended_at, deleted_at FROM prescriptions WHERE id = ?1",
        params![rx_id],
        |row| Ok((row.get(0)?, row.get(1)?)),
    ) {
        Ok(t) => t,
        Err(rusqlite::Error::QueryReturnedNoRows) => {
            return Err(PillboxError::PrescriptionNotFound {
                id: rx_id.to_string(),
            }
            .into());
        }
        Err(e) => return Err(e).context("failed to verify prescription state"),
    };

    if ended_at.is_some() || deleted_at.is_some() {
        return Err(PillboxError::PrescriptionClosed {
            prescription_id: rx_id.to_string(),
        }
        .into());
    }
    Ok(())
}

/// Soft delete de una prescription y todas sus pills.
///
/// Acepta UUID completo o prefijo ≥8 chars. El cascade es lógico: establece
/// `deleted_at` en la prescription y en todas sus pills activas. No borra
/// filas de forma permanente.
///
/// # Errors
///
/// Devuelve [`PillboxError::PrescriptionNotFound`] si la prescription no existe
/// o ya estaba descartada.
pub fn discard(conn: &mut Connection, id: &str) -> Result<()> {
    let resolved_id = resolve_id(conn, "prescriptions", id)?
        .ok_or_else(|| PillboxError::PrescriptionNotFound { id: id.to_string() })?;

    let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;

    let affected = tx.execute(
        "UPDATE prescriptions SET deleted_at = datetime('now')
         WHERE id = ?1 AND deleted_at IS NULL",
        params![resolved_id],
    )?;

    if affected == 0 {
        return Err(PillboxError::PrescriptionNotFound { id: id.to_string() }.into());
    }

    tx.execute(
        "UPDATE pills SET deleted_at = datetime('now')
         WHERE prescription_id = ?1 AND deleted_at IS NULL",
        params![resolved_id],
    )?;

    tx.commit()?;
    Ok(())
}

/// Hard delete de una prescription y todos sus datos relacionados (irreversible).
///
/// Acepta UUID completo o prefijo ≥8 chars. Elimina en orden dentro de una
/// tx IMMEDIATE: pills → prescriptions.
///
/// Falla con `PillboxError::PrescriptionNotFound` si la prescription no existe.
pub fn hard_delete(conn: &mut Connection, id: &str) -> Result<()> {
    let resolved_id = resolve_id(conn, "prescriptions", id)?
        .ok_or_else(|| PillboxError::PrescriptionNotFound { id: id.to_string() })?;

    let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;

    // resolve_id puede hacer short-circuit con un UUID completo sin verificar
    // que exista, así que comprobamos existencia explícita dentro de la tx.
    let exists: bool = tx.query_row(
        "SELECT EXISTS(SELECT 1 FROM prescriptions WHERE id = ?1)",
        params![resolved_id],
        |row| row.get(0),
    )?;

    if !exists {
        return Err(PillboxError::PrescriptionNotFound { id: id.to_string() }.into());
    }

    // 1. Eliminar pills de la prescription
    tx.execute(
        "DELETE FROM pills WHERE prescription_id = ?1",
        params![resolved_id],
    )
    .context("failed to delete pills for prescription")?;

    // 2. Eliminar la prescription
    tx.execute("DELETE FROM prescriptions WHERE id = ?1", params![resolved_id])
        .context("failed to delete prescription")?;

    tx.commit()?;
    Ok(())
}

/// Lee una prescription por UUID completo o prefijo ≥8 chars, incluyendo descartadas.
pub fn read_any(conn: &Connection, id: &str) -> Result<Option<Prescription>> {
    let Some(resolved_id) = resolve_id(conn, "prescriptions", id)? else {
        return Ok(None);
    };
    match conn.query_row(
        "SELECT id, bottle_id, title, author_name, author_email, started_at, ended_at, deleted_at
         FROM prescriptions
         WHERE id = ?1",
        params![resolved_id],
        row_to_prescription,
    ) {
        Ok(rx) => Ok(Some(rx)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(e).context("failed to read prescription"),
    }
}

/// Lee una prescription por UUID completo o prefijo ≥8 chars (no descartada).
pub fn read(conn: &Connection, id: &str) -> Result<Option<Prescription>> {
    let Some(resolved_id) = resolve_id(conn, "prescriptions", id)? else {
        return Ok(None);
    };
    match conn.query_row(
        "SELECT id, bottle_id, title, author_name, author_email, started_at, ended_at, deleted_at
         FROM prescriptions
         WHERE id = ?1 AND deleted_at IS NULL",
        params![resolved_id],
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

/// Lista prescriptions del bottle aplicando el filtro de estado indicado.
///
/// El orden es siempre `started_at DESC` para que las más recientes aparezcan
/// primero. El parámetro `filter` controla la inclusión de archivadas.
pub fn list_by_bottle(
    conn: &Connection,
    bottle_id: &str,
    limit: u32,
    filter: ListFilter,
) -> Result<Vec<Prescription>> {
    let filter_clause = match filter {
        ListFilter::Active => "AND deleted_at IS NULL",
        ListFilter::Archived => "AND deleted_at IS NOT NULL",
        ListFilter::All => "",
    };
    let sql = format!(
        "SELECT id, bottle_id, title, author_name, author_email, started_at, ended_at, deleted_at
         FROM prescriptions
         WHERE bottle_id = ?1 {}
         ORDER BY started_at DESC
         LIMIT ?2",
        filter_clause,
    );
    let mut stmt = conn.prepare(&sql)?;

    let rows = stmt
        .query_map(params![bottle_id, limit], row_to_prescription)?
        .collect::<rusqlite::Result<Vec<_>>>()
        .context("failed to list prescriptions")?;

    Ok(rows)
}

/// Cuenta las pills archivadas de una prescription.
///
/// Usado por el comando `prescription show` para mostrar el trailer
/// exacto cuando hay más archivadas que el límite del usuario.
pub fn count_archived_pills(conn: &Connection, prescription_id: &str) -> Result<u32> {
    let n: u32 = conn.query_row(
        "SELECT COUNT(*) FROM pills
         WHERE prescription_id = ?1 AND deleted_at IS NOT NULL",
        params![prescription_id],
        |r| r.get(0),
    )?;
    Ok(n)
}

/// Cuenta las prescriptions archivadas de un bottle.
///
/// Necesario para que el trailer `... N más archivados` muestre el número
/// exacto de elementos ocultos, en vez de un genérico.
pub fn count_archived_by_bottle(conn: &Connection, bottle_id: &str) -> Result<u32> {
    let n: u32 = conn.query_row(
        "SELECT COUNT(*) FROM prescriptions
         WHERE bottle_id = ?1 AND deleted_at IS NOT NULL",
        params![bottle_id],
        |r| r.get(0),
    )?;
    Ok(n)
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

    fn make_rx(conn: &mut Connection, bottle_id: &str, title: &str) -> String {
        open(
            conn,
            &NewPrescription {
                bottle_id: bottle_id.into(),
                title: title.into(),
                author_name: None,
                author_email: None,
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
    fn open_resolves_12char_bottle_prefix() {
        let mut conn = open_in_memory().unwrap();
        let bottle_id = make_bottle(&mut conn, "short-id-test");
        let short = bottle_id.replace('-', "").chars().take(12).collect::<String>();

        let rx = open(
            &mut conn,
            &NewPrescription {
                bottle_id: short,
                title: "Con short bottle_id".into(),
                author_name: None,
                author_email: None,
            },
        )
        .unwrap();

        // La prescription debe almacenar el UUID completo del bottle, no el prefijo
        assert_eq!(rx.bottle_id, bottle_id);
    }

    #[test]
    fn open_resolves_8char_bottle_prefix() {
        let mut conn = open_in_memory().unwrap();
        let bottle_id = make_bottle(&mut conn, "short8-test");
        let short = bottle_id.replace('-', "").chars().take(8).collect::<String>();

        let rx = open(
            &mut conn,
            &NewPrescription {
                bottle_id: short,
                title: "8 chars".into(),
                author_name: None,
                author_email: None,
            },
        )
        .unwrap();
        assert_eq!(rx.bottle_id, bottle_id);
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

        let all = list_by_bottle(&conn, &bottle_id, 10, ListFilter::All).unwrap();
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

        let limited = list_by_bottle(&conn, &bottle_id, 3, ListFilter::All).unwrap();
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

        conn.execute(
            "INSERT INTO pills (id, compound, title, content, prescription_id)
             VALUES ('01900000-0000-7000-0000-000000000001', 'decision', 'T', 'C', ?1)",
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

        conn.execute(
            "INSERT INTO pills (id, compound, title, content, prescription_id)
             VALUES ('01900000-0000-7000-0000-000000000002', 'decision', 'T', 'C', ?1)",
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

        let all = list_by_bottle(&conn, &bottle_id, 10, ListFilter::All).unwrap();
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

        let all = list_by_bottle(&conn, &bottle_id, 10, ListFilter::All).unwrap();
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

        let listed = list_by_bottle(&conn, &rx.bottle_id, 10, ListFilter::All).unwrap();
        assert_eq!(listed[0].author_name.as_deref(), Some("Kevin Illanas"));
        assert_eq!(listed[0].author_email.as_deref(), Some("kevin@example.com"));
    }

    #[test]
    fn list_active_filter_excludes_archived() {
        let mut conn = open_in_memory().unwrap();
        let bottle_id = make_bottle(&mut conn, "active-filter");
        for i in 0..3 {
            let id = make_rx(&mut conn, &bottle_id, &format!("Active {i}"));
            close(&mut conn, &id).unwrap();
        }
        for i in 0..5 {
            let id = make_rx(&mut conn, &bottle_id, &format!("Archived {i}"));
            discard(&mut conn, &id).unwrap();
        }
        let active = list_by_bottle(&conn, &bottle_id, 100, ListFilter::Active).unwrap();
        assert_eq!(active.len(), 3);
        assert!(active.iter().all(|rx| rx.deleted_at.is_none()));
    }

    #[test]
    fn list_archived_filter_excludes_active() {
        let mut conn = open_in_memory().unwrap();
        let bottle_id = make_bottle(&mut conn, "archived-filter");
        for i in 0..3 {
            let id = make_rx(&mut conn, &bottle_id, &format!("Active {i}"));
            close(&mut conn, &id).unwrap();
        }
        for i in 0..5 {
            let id = make_rx(&mut conn, &bottle_id, &format!("Archived {i}"));
            discard(&mut conn, &id).unwrap();
        }
        let archived = list_by_bottle(&conn, &bottle_id, 100, ListFilter::Archived).unwrap();
        assert_eq!(archived.len(), 5);
        assert!(archived.iter().all(|rx| rx.deleted_at.is_some()));
    }

    #[test]
    fn count_archived_by_bottle_returns_only_archived() {
        let mut conn = open_in_memory().unwrap();
        let bottle_id = make_bottle(&mut conn, "count-archived");
        for i in 0..4 {
            let id = make_rx(&mut conn, &bottle_id, &format!("Active {i}"));
            close(&mut conn, &id).unwrap();
        }
        for i in 0..8 {
            let id = make_rx(&mut conn, &bottle_id, &format!("Archived {i}"));
            discard(&mut conn, &id).unwrap();
        }
        let count = count_archived_by_bottle(&conn, &bottle_id).unwrap();
        assert_eq!(count, 8);
    }

    #[test]
    fn archived_limit_clamps_returned_rows() {
        let mut conn = open_in_memory().unwrap();
        let bottle_id = make_bottle(&mut conn, "limit-clamp");
        for i in 0..8 {
            let id = make_rx(&mut conn, &bottle_id, &format!("Archived {i}"));
            discard(&mut conn, &id).unwrap();
        }
        let archived = list_by_bottle(&conn, &bottle_id, 5, ListFilter::Archived).unwrap();
        assert_eq!(archived.len(), 5);
        let total = count_archived_by_bottle(&conn, &bottle_id).unwrap();
        assert_eq!(total, 8);
    }

    #[test]
    fn read_by_12char_prefix() {
        let mut conn = open_in_memory().unwrap();
        let bottle_id = make_bottle(&mut conn, "prefix-rx-12");
        let rx_id = make_rx(&mut conn, &bottle_id, "Test session");
        let short = rx_id.replace('-', "").chars().take(12).collect::<String>();
        let found = read(&conn, &short).unwrap().unwrap();
        assert_eq!(found.id, rx_id);
    }

    #[test]
    fn close_by_short_id() {
        let mut conn = open_in_memory().unwrap();
        let bottle_id = make_bottle(&mut conn, "close-short-rx");
        let rx_id = make_rx(&mut conn, &bottle_id, "Closing session");
        let short = rx_id.replace('-', "").chars().take(12).collect::<String>();
        let closed = close(&mut conn, &short).unwrap();
        assert!(closed.ended_at.is_some());
    }

    #[test]
    fn discard_by_short_id() {
        let mut conn = open_in_memory().unwrap();
        let bottle_id = make_bottle(&mut conn, "discard-short-rx");
        let rx_id = make_rx(&mut conn, &bottle_id, "Discard session");
        let short = rx_id.replace('-', "").chars().take(12).collect::<String>();
        discard(&mut conn, &short).unwrap();
        assert!(read(&conn, &rx_id).unwrap().is_none());
    }

    #[test]
    fn read_too_short_returns_invalid_id() {
        let conn = open_in_memory().unwrap();
        let err = read(&conn, "abc").unwrap_err();
        let typed = err.downcast_ref::<PillboxError>().unwrap();
        assert!(matches!(typed, PillboxError::InvalidId { .. }));
    }

    #[test]
    fn read_ambiguous_returns_ambiguous_id() {
        let mut conn = open_in_memory().unwrap();
        let bottle_id = make_bottle(&mut conn, "amb-rx");
        // Primera rx cerrada (ended_at != NULL) para no violar el índice UNIQUE parcial
        // que solo aplica a prescriptions con ended_at IS NULL.
        conn.execute(
            "INSERT INTO prescriptions (id, bottle_id, title, ended_at)
             VALUES ('01234567-aaaa-7000-8000-000000000001', ?1, 'A', datetime('now'))",
            params![bottle_id],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO prescriptions (id, bottle_id, title)
             VALUES ('01234567-aaaa-7000-8000-000000000002', ?1, 'B')",
            params![bottle_id],
        )
        .unwrap();
        let err = read(&conn, "01234567aaaa").unwrap_err();
        let typed = err.downcast_ref::<PillboxError>().unwrap();
        assert!(matches!(typed, PillboxError::AmbiguousId { .. }));
    }

    #[test]
    fn reopen_closed_prescription_no_collision() {
        let mut conn = open_in_memory().unwrap();
        let bottle_id = make_bottle(&mut conn, "reopen-basic");
        let rx_id = make_rx(&mut conn, &bottle_id, "Reopen me");
        close(&mut conn, &rx_id).unwrap();

        let reopened = reopen(&mut conn, &rx_id).unwrap();
        assert_eq!(reopened.id, rx_id);
        assert!(reopened.ended_at.is_none());
        assert!(reopened.deleted_at.is_none());

        // verificar persistido
        let found = read(&conn, &rx_id).unwrap().unwrap();
        assert!(found.ended_at.is_none());
    }

    #[test]
    fn reopen_collision_in_bottle() {
        let mut conn = open_in_memory().unwrap();
        let bottle_id = make_bottle(&mut conn, "reopen-collision");

        // Prescription A: abierta y luego cerrada
        let rx_a = make_rx(&mut conn, &bottle_id, "A");
        close(&mut conn, &rx_a).unwrap();

        // Prescription B: abierta (única abierta actualmente)
        let rx_b = make_rx(&mut conn, &bottle_id, "B");

        // Intentar reabrir A → colisión con B
        let err = reopen(&mut conn, &rx_a).unwrap_err();
        let typed = err.downcast_ref::<PillboxError>().unwrap();
        match typed {
            PillboxError::PrescriptionAlreadyOpenInBottle {
                bottle_id: bid,
                existing_id,
            } => {
                assert_eq!(bid, &bottle_id);
                assert_eq!(existing_id, &rx_b);
            }
            other => panic!("expected PrescriptionAlreadyOpenInBottle, got {:?}", other),
        }

        // Verificar que A sigue cerrada
        let a_found = read(&conn, &rx_a).unwrap().unwrap();
        assert!(a_found.ended_at.is_some());
    }

    #[test]
    fn reopen_already_open() {
        let mut conn = open_in_memory().unwrap();
        let bottle_id = make_bottle(&mut conn, "reopen-already-open");
        let rx_id = make_rx(&mut conn, &bottle_id, "Already open");

        let err = reopen(&mut conn, &rx_id).unwrap_err();
        let typed = err.downcast_ref::<PillboxError>().unwrap();
        assert!(matches!(typed, PillboxError::PrescriptionAlreadyOpen { .. }));
    }

    #[test]
    fn reopen_not_found() {
        let mut conn = open_in_memory().unwrap();
        let err = reopen(&mut conn, "00000000-0000-0000-0000-000000000000").unwrap_err();
        let typed = err.downcast_ref::<PillboxError>().unwrap();
        assert!(matches!(typed, PillboxError::PrescriptionNotFound { .. }));
    }

    #[test]
    fn count_archived_pills_returns_exact_total() {
        use crate::db::store::pills;
        use crate::domain::pill::NewPill;

        let mut conn = open_in_memory().unwrap();
        let bottle_id = make_bottle(&mut conn, "pills-count");
        let rx_id = make_rx(&mut conn, &bottle_id, "rx con pills");
        for i in 0..2 {
            pills::take(
                &mut conn,
                &NewPill {
                    title: format!("active {i}"),
                    content: "contenido".into(),
                    compound: "decision".into(),
                    prescription_id: rx_id.clone(),
                    author_name: None,
                    author_email: None,
                },
            )
            .unwrap();
        }
        let mut archived_ids = Vec::new();
        for i in 0..7 {
            let p = pills::take(
                &mut conn,
                &NewPill {
                    title: format!("archived {i}"),
                    content: "contenido".into(),
                    compound: "decision".into(),
                    prescription_id: rx_id.clone(),
                    author_name: None,
                    author_email: None,
                },
            )
            .unwrap();
            archived_ids.push(p.id);
        }
        for id in &archived_ids {
            pills::discard(&mut conn, id).unwrap();
        }
        let count = count_archived_pills(&conn, &rx_id).unwrap();
        assert_eq!(count, 7);
    }
}
