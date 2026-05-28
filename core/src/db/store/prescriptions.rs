//! Operaciones de store para la entidad [`Prescription`].

use anyhow::{Context, Result};
use rusqlite::{params, Connection, TransactionBehavior};
use uuid::Uuid;

use crate::db::store::id_resolver::resolve_id;
use crate::db::store::ListFilter;
use crate::domain::prescription::{NewPrescription, Prescription};
use crate::domain::{Paginated, PaginationParams};
use crate::error::PillboxError;

// ─── Store ────────────────────────────────────────────────────────────────────

/// Abre una nueva prescription para el bottle dado.
///
/// Múltiples prescriptions abiertas simultáneamente para el mismo bottle son
/// válidas: cada `prescription_open` crea una nueva fila sin colisionar con
/// otras prescriptions activas.
pub fn open(conn: &mut Connection, input: &NewPrescription) -> Result<Prescription> {
    // Resolver el bottle_id (acepta UUID completo o prefijo ≥8 chars) y validar
    // que existe antes de abrir la prescription.
    let resolved_bottle_id = resolve_id(conn, "bottles", &input.bottle_id)?.ok_or_else(|| {
        PillboxError::BottleNotFound {
            bottle_id: input.bottle_id.clone(),
        }
    })?;

    let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;

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
            "SELECT id, bottle_id, title, author_name, author_email, started_at, ended_at, deleted_at,
                views
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
        .ok_or_else(|| PillboxError::PrescriptionNotFound { id: id.to_string() })?;

    let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;

    let affected = tx
        .execute(
            "UPDATE prescriptions SET ended_at = datetime('now')
         WHERE id = ?1 AND ended_at IS NULL AND deleted_at IS NULL",
            params![resolved_id],
        )
        .context("failed to close prescription")?;

    if affected == 0 {
        return Err(PillboxError::PrescriptionNotFound { id: id.to_string() }.into());
    }

    let prescription = tx
        .query_row(
            "SELECT id, bottle_id, title, author_name, author_email, started_at, ended_at, deleted_at,
                views
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
/// Reopen sobre una prescription ya abierta es un no-op idempotente: devuelve
/// la prescription tal cual sin modificar timestamps.
///
/// # Errores
/// - [`PillboxError::PrescriptionNotFound`] si no existe (incluye prefijo no resoluble).
/// - [`PillboxError::PrescriptionNotFound`] si está descartada (`deleted_at IS NOT NULL`).
pub fn reopen(conn: &mut Connection, id: &str) -> Result<Prescription> {
    let resolved_id = resolve_id(conn, "prescriptions", id)?
        .ok_or_else(|| PillboxError::PrescriptionNotFound { id: id.to_string() })?;

    // Pre-check fuera de tx: existencia + estado actual.
    let (ended_at, deleted_at): (Option<String>, Option<String>) = match conn.query_row(
        "SELECT ended_at, deleted_at FROM prescriptions WHERE id = ?1",
        params![resolved_id],
        |row| Ok((row.get(0)?, row.get(1)?)),
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
        // Ya está abierta — no-op idempotente: devolvemos la rx tal cual sin tocar timestamps.
        let prescription = conn
            .query_row(
                "SELECT id, bottle_id, title, author_name, author_email, started_at, ended_at, deleted_at,
                        views
                 FROM prescriptions WHERE id = ?1",
                params![resolved_id],
                row_to_prescription,
            )
            .context("failed to read already-open prescription")?;
        return Ok(prescription);
    }

    // Tx IMMEDIATE: UPDATE + lectura final.
    let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;

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
            "SELECT id, bottle_id, title, author_name, author_email, started_at, ended_at, deleted_at,
                    views
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
        "UPDATE prescriptions
         SET deleted_at = datetime('now'),
             ended_at = COALESCE(ended_at, datetime('now'))
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
    tx.execute(
        "DELETE FROM prescriptions WHERE id = ?1",
        params![resolved_id],
    )
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
        "SELECT id, bottle_id, title, author_name, author_email, started_at, ended_at, deleted_at,
                views
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
        "SELECT id, bottle_id, title, author_name, author_email, started_at, ended_at, deleted_at,
                views
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

/// Cuenta las prescriptions abiertas (sin `ended_at`, no descartadas) de un bottle.
pub fn count_open_by_bottle(conn: &Connection, bottle_id: &str) -> Result<u32> {
    let n: u32 = conn.query_row(
        "SELECT COUNT(*) FROM prescriptions
         WHERE bottle_id = ?1 AND ended_at IS NULL AND deleted_at IS NULL",
        params![bottle_id],
        |r| r.get(0),
    )?;
    Ok(n)
}

/// Lista las prescriptions abiertas (sin `ended_at`, no descartadas) de un
/// bottle como pares `(id, title)`, más recientes primero.
pub fn list_open_by_bottle(conn: &Connection, bottle_id: &str) -> Result<Vec<(String, String)>> {
    let mut stmt = conn.prepare(
        "SELECT id, title FROM prescriptions
         WHERE bottle_id = ?1 AND ended_at IS NULL AND deleted_at IS NULL
         ORDER BY started_at DESC",
    )?;
    let rows = stmt
        .query_map(params![bottle_id], |row| Ok((row.get(0)?, row.get(1)?)))?
        .collect::<rusqlite::Result<Vec<_>>>()
        .context("failed to list open prescriptions by bottle")?;
    Ok(rows)
}

/// Cuenta las prescriptions activas (no descartadas) de un bottle.
///
/// Acepta el `bottle_id` resuelto (UUID completo) o el `bottle_pattern`
/// LIKE — el caller debe pasar el id ya normalizado a través de
/// `id_resolver::normalize_prefix` cuando esté trabajando con prefijos.
pub fn count_active_by_bottle(conn: &Connection, bottle_id: &str) -> Result<u64> {
    let bottle_pattern = format!("{}%", bottle_id);
    let n: u64 = conn.query_row(
        "SELECT COUNT(*) FROM prescriptions
         WHERE (bottle_id = ?1 OR bottle_id LIKE ?2)
           AND deleted_at IS NULL",
        params![bottle_id, bottle_pattern],
        |r| r.get(0),
    )?;
    Ok(n)
}

/// Lista prescriptions del bottle (paginado, orden `started_at DESC`).
///
/// El filtro `ListFilter` decide si se devuelven activas, archivadas o todas.
pub fn list_by_bottle(
    conn: &Connection,
    bottle_id: &str,
    filter: ListFilter,
    pagination: &PaginationParams,
) -> Result<Paginated<Prescription>> {
    let filter_clause = match filter {
        ListFilter::Active => "deleted_at IS NULL",
        ListFilter::Archived => "deleted_at IS NOT NULL",
        ListFilter::All => "1=1",
    };

    let count_sql = format!(
        "SELECT COUNT(*) FROM prescriptions
         WHERE bottle_id = ?1 AND {filter_clause}"
    );
    let total: u64 = conn.query_row(&count_sql, params![bottle_id], |r| r.get(0))?;

    let limit = pagination.limit() as i64;
    let offset = pagination.offset() as i64;

    let select_sql = format!(
        "SELECT id, bottle_id, title, author_name, author_email, started_at, ended_at, deleted_at,
                views
         FROM prescriptions
         WHERE bottle_id = ?1 AND {filter_clause}
         ORDER BY started_at DESC
         LIMIT ?2 OFFSET ?3"
    );
    let mut stmt = conn.prepare(&select_sql)?;

    let items = stmt
        .query_map(params![bottle_id, limit, offset], row_to_prescription)?
        .collect::<rusqlite::Result<Vec<_>>>()
        .context("failed to list prescriptions")?;

    Ok(Paginated {
        items,
        total,
        page: pagination.page,
        page_size: pagination.page_size,
        used_fuzzy: false,
    })
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
        views: row.get(8)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::connection::open_in_memory;
    use crate::db::store::bottles;
    use crate::db::DbScope;
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
        let mut conn = open_in_memory(DbScope::Local).unwrap();
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
    fn multiple_open_prescriptions_per_bottle_allowed() {
        let mut conn = open_in_memory(DbScope::Local).unwrap();
        let bottle_id = make_bottle(&mut conn, "multi-open");

        let rx_a = open(
            &mut conn,
            &NewPrescription {
                bottle_id: bottle_id.clone(),
                title: "Primera sesión".into(),
                author_name: None,
                author_email: None,
            },
        )
        .unwrap();

        let rx_b = open(
            &mut conn,
            &NewPrescription {
                bottle_id: bottle_id.clone(),
                title: "Segunda sesión".into(),
                author_name: None,
                author_email: None,
            },
        )
        .unwrap();

        assert_ne!(rx_a.id, rx_b.id);
        assert!(rx_a.ended_at.is_none());
        assert!(rx_b.ended_at.is_none());

        // Ambas deben aparecer listadas como activas.
        let listed = list_by_bottle(
            &conn,
            &bottle_id,
            ListFilter::Active,
            &PaginationParams {
                page: 1,
                page_size: 10,
            },
        )
        .unwrap();
        assert_eq!(listed.items.len(), 2);
        assert!(listed.items.iter().all(|rx| rx.ended_at.is_none()));
    }

    #[test]
    fn multiple_open_prescriptions_per_bottle_allowed_global() {
        // Regresión: el schema global tenía un UNIQUE index parcial (idx_rx_open)
        // que rechazaba la segunda prescription abierta por bottle. Este test
        // garantiza que bottles registrados en la DB global también soportan
        // múltiples prescriptions abiertas simultáneamente.
        let mut conn = open_in_memory(DbScope::Global).unwrap();
        let bottle_id = make_bottle(&mut conn, "multi-open-global");

        let rx_a = open(
            &mut conn,
            &NewPrescription {
                bottle_id: bottle_id.clone(),
                title: "Primera sesión".into(),
                author_name: None,
                author_email: None,
            },
        )
        .unwrap();

        let rx_b = open(
            &mut conn,
            &NewPrescription {
                bottle_id: bottle_id.clone(),
                title: "Segunda sesión".into(),
                author_name: None,
                author_email: None,
            },
        )
        .unwrap();

        assert_ne!(rx_a.id, rx_b.id);
        assert!(rx_a.ended_at.is_none());
        assert!(rx_b.ended_at.is_none());
    }

    #[test]
    fn bottle_not_found_fails() {
        let mut conn = open_in_memory(DbScope::Local).unwrap();
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
        let mut conn = open_in_memory(DbScope::Local).unwrap();
        let bottle_id = make_bottle(&mut conn, "short-id-test");
        let short = bottle_id
            .replace('-', "")
            .chars()
            .take(12)
            .collect::<String>();

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
        let mut conn = open_in_memory(DbScope::Local).unwrap();
        let bottle_id = make_bottle(&mut conn, "short8-test");
        let short = bottle_id
            .replace('-', "")
            .chars()
            .take(8)
            .collect::<String>();

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
        let mut conn = open_in_memory(DbScope::Local).unwrap();
        assert!(close(&mut conn, "id-inexistente").is_err());
    }

    #[test]
    fn read_existing_prescription() {
        let mut conn = open_in_memory(DbScope::Local).unwrap();
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
        let mut conn = open_in_memory(DbScope::Local).unwrap();
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
        let mut conn = open_in_memory(DbScope::Local).unwrap();
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

        let all = list_by_bottle(
            &conn,
            &bottle_id,
            ListFilter::Active,
            &PaginationParams {
                page: 1,
                page_size: 10,
            },
        )
        .unwrap();
        assert_eq!(all.items.len(), 2);
        let titles: Vec<&str> = all.items.iter().map(|r| r.title.as_str()).collect();
        assert!(titles.contains(&"Sesión 1"));
        assert!(titles.contains(&"Sesión 2"));
    }

    #[test]
    fn list_by_bottle_respects_limit() {
        let mut conn = open_in_memory(DbScope::Local).unwrap();
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

        let limited = list_by_bottle(
            &conn,
            &bottle_id,
            ListFilter::Active,
            &PaginationParams {
                page: 1,
                page_size: 3,
            },
        )
        .unwrap();
        assert_eq!(limited.items.len(), 3);
    }

    #[test]
    fn discard_twice_fails() {
        let mut conn = open_in_memory(DbScope::Local).unwrap();
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
    fn discard_open_prescription_also_closes_it() {
        let mut conn = open_in_memory(DbScope::Local).unwrap();
        let bottle_id = make_bottle(&mut conn, "discard-open");
        let rx = open(
            &mut conn,
            &NewPrescription {
                bottle_id,
                title: "Abierta a descartar".into(),
                author_name: None,
                author_email: None,
            },
        )
        .unwrap();
        assert!(rx.ended_at.is_none());

        discard(&mut conn, &rx.id).unwrap();

        let stored = read_any(&conn, &rx.id).unwrap().unwrap();
        assert!(stored.deleted_at.is_some());
        assert!(
            stored.ended_at.is_some(),
            "discard sobre rx abierta debe marcar ended_at",
        );
    }

    #[test]
    fn discard_closed_prescription_preserves_ended_at() {
        let mut conn = open_in_memory(DbScope::Local).unwrap();
        let bottle_id = make_bottle(&mut conn, "discard-closed");
        let rx = open(
            &mut conn,
            &NewPrescription {
                bottle_id,
                title: "Cerrada antes de descartar".into(),
                author_name: None,
                author_email: None,
            },
        )
        .unwrap();
        let closed = close(&mut conn, &rx.id).unwrap();
        let original_ended_at = closed.ended_at.clone().unwrap();

        discard(&mut conn, &rx.id).unwrap();

        let stored = read_any(&conn, &rx.id).unwrap().unwrap();
        assert_eq!(
            stored.ended_at.as_deref(),
            Some(original_ended_at.as_str()),
            "discard no debe reescribir ended_at si ya estaba cerrada",
        );
    }

    #[test]
    fn hard_delete_prescription_removes_all_rows() {
        let mut conn = open_in_memory(DbScope::Local).unwrap();
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
            .query_row(
                "SELECT COUNT(*) FROM prescriptions WHERE id = ?1",
                params![rx.id],
                |r| r.get(0),
            )
            .unwrap();
        let pill_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM pills WHERE prescription_id = ?1",
                params![rx.id],
                |r| r.get(0),
            )
            .unwrap();

        assert_eq!(rx_count, 0);
        assert_eq!(pill_count, 0);
    }

    #[test]
    fn hard_delete_nonexistent_prescription_returns_error() {
        let mut conn = open_in_memory(DbScope::Local).unwrap();
        let err = hard_delete(&mut conn, "id-inexistente").unwrap_err();
        let typed = err.downcast_ref::<PillboxError>();
        assert!(matches!(
            typed,
            Some(PillboxError::PrescriptionNotFound { .. })
        ));
    }

    #[test]
    fn discard_cascades_to_pills() {
        let mut conn = open_in_memory(DbScope::Local).unwrap();
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
    fn list_by_bottle_excludes_archived() {
        let mut conn = open_in_memory(DbScope::Local).unwrap();
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

        let all = list_by_bottle(
            &conn,
            &bottle_id,
            ListFilter::Active,
            &PaginationParams {
                page: 1,
                page_size: 10,
            },
        )
        .unwrap();
        assert!(all.items.is_empty());
        assert_eq!(all.total, 0);
    }

    #[test]
    fn list_by_bottle_only_active() {
        let mut conn = open_in_memory(DbScope::Local).unwrap();
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

        // Archived prescription
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

        let all = list_by_bottle(
            &conn,
            &bottle_id,
            ListFilter::Active,
            &PaginationParams {
                page: 1,
                page_size: 10,
            },
        )
        .unwrap();
        assert_eq!(all.items.len(), 1);
        assert_eq!(all.total, 1);
        assert!(all.items.iter().all(|rx| rx.deleted_at.is_none()));
    }

    #[test]
    fn read_any_returns_archived_prescription() {
        let mut conn = open_in_memory(DbScope::Local).unwrap();
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
        let mut conn = open_in_memory(DbScope::Local).unwrap();
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
        let mut conn = open_in_memory(DbScope::Local).unwrap();
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

        let listed = list_by_bottle(
            &conn,
            &rx.bottle_id,
            ListFilter::Active,
            &PaginationParams {
                page: 1,
                page_size: 10,
            },
        )
        .unwrap();
        assert_eq!(
            listed.items[0].author_name.as_deref(),
            Some("Kevin Illanas")
        );
        assert_eq!(
            listed.items[0].author_email.as_deref(),
            Some("kevin@example.com")
        );
    }

    #[test]
    fn list_paginates_active_only() {
        let mut conn = open_in_memory(DbScope::Local).unwrap();
        let bottle_id = make_bottle(&mut conn, "active-filter");
        for i in 0..3 {
            let id = make_rx(&mut conn, &bottle_id, &format!("Active {i}"));
            close(&mut conn, &id).unwrap();
        }
        for i in 0..5 {
            let id = make_rx(&mut conn, &bottle_id, &format!("Archived {i}"));
            discard(&mut conn, &id).unwrap();
        }
        let active = list_by_bottle(
            &conn,
            &bottle_id,
            ListFilter::Active,
            &PaginationParams {
                page: 1,
                page_size: 100,
            },
        )
        .unwrap();
        assert_eq!(active.items.len(), 3);
        assert_eq!(active.total, 3);
        assert!(active.items.iter().all(|rx| rx.deleted_at.is_none()));
    }

    #[test]
    fn count_archived_by_bottle_returns_only_archived() {
        let mut conn = open_in_memory(DbScope::Local).unwrap();
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
    fn count_archived_by_bottle_clamps_independent() {
        let mut conn = open_in_memory(DbScope::Local).unwrap();
        let bottle_id = make_bottle(&mut conn, "limit-clamp");
        for i in 0..8 {
            let id = make_rx(&mut conn, &bottle_id, &format!("Archived {i}"));
            discard(&mut conn, &id).unwrap();
        }
        let total = count_archived_by_bottle(&conn, &bottle_id).unwrap();
        assert_eq!(total, 8);
    }

    #[test]
    fn read_by_12char_prefix() {
        let mut conn = open_in_memory(DbScope::Local).unwrap();
        let bottle_id = make_bottle(&mut conn, "prefix-rx-12");
        let rx_id = make_rx(&mut conn, &bottle_id, "Test session");
        let short = rx_id.replace('-', "").chars().take(12).collect::<String>();
        let found = read(&conn, &short).unwrap().unwrap();
        assert_eq!(found.id, rx_id);
    }

    #[test]
    fn close_by_short_id() {
        let mut conn = open_in_memory(DbScope::Local).unwrap();
        let bottle_id = make_bottle(&mut conn, "close-short-rx");
        let rx_id = make_rx(&mut conn, &bottle_id, "Closing session");
        let short = rx_id.replace('-', "").chars().take(12).collect::<String>();
        let closed = close(&mut conn, &short).unwrap();
        assert!(closed.ended_at.is_some());
    }

    #[test]
    fn discard_by_short_id() {
        let mut conn = open_in_memory(DbScope::Local).unwrap();
        let bottle_id = make_bottle(&mut conn, "discard-short-rx");
        let rx_id = make_rx(&mut conn, &bottle_id, "Discard session");
        let short = rx_id.replace('-', "").chars().take(12).collect::<String>();
        discard(&mut conn, &short).unwrap();
        assert!(read(&conn, &rx_id).unwrap().is_none());
    }

    #[test]
    fn read_too_short_returns_invalid_id() {
        let conn = open_in_memory(DbScope::Local).unwrap();
        let err = read(&conn, "abc").unwrap_err();
        let typed = err.downcast_ref::<PillboxError>().unwrap();
        assert!(matches!(typed, PillboxError::InvalidId { .. }));
    }

    #[test]
    fn read_ambiguous_returns_ambiguous_id() {
        let mut conn = open_in_memory(DbScope::Local).unwrap();
        let bottle_id = make_bottle(&mut conn, "amb-rx");
        // Dos prescriptions con el mismo prefijo (12 hex). Múltiples rx abiertas
        // por bottle están permitidas tras eliminar el índice idx_rx_open.
        conn.execute(
            "INSERT INTO prescriptions (id, bottle_id, title)
             VALUES ('01234567-aaaa-7000-8000-000000000001', ?1, 'A')",
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
        let mut conn = open_in_memory(DbScope::Local).unwrap();
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
    fn reopen_with_another_open_in_same_bottle_succeeds() {
        let mut conn = open_in_memory(DbScope::Local).unwrap();
        let bottle_id = make_bottle(&mut conn, "reopen-multi");

        // Prescription A: abierta y luego cerrada
        let rx_a = make_rx(&mut conn, &bottle_id, "A");
        close(&mut conn, &rx_a).unwrap();

        // Prescription B: abierta (coexiste con A reabierta)
        let rx_b = make_rx(&mut conn, &bottle_id, "B");

        // Reabrir A debe funcionar incluso con B abierta en el mismo bottle.
        let reopened = reopen(&mut conn, &rx_a).unwrap();
        assert_eq!(reopened.id, rx_a);
        assert!(reopened.ended_at.is_none());

        // B sigue abierta.
        let b_found = read(&conn, &rx_b).unwrap().unwrap();
        assert!(b_found.ended_at.is_none());
    }

    #[test]
    fn reopen_already_open_is_noop() {
        let mut conn = open_in_memory(DbScope::Local).unwrap();
        let bottle_id = make_bottle(&mut conn, "reopen-already-open");
        let rx_id = make_rx(&mut conn, &bottle_id, "Already open");

        let before = read(&conn, &rx_id).unwrap().unwrap();
        let reopened = reopen(&mut conn, &rx_id).unwrap();
        assert_eq!(reopened.id, rx_id);
        assert!(reopened.ended_at.is_none());
        // No-op: no toca started_at.
        assert_eq!(reopened.started_at, before.started_at);
    }

    #[test]
    fn reopen_not_found() {
        let mut conn = open_in_memory(DbScope::Local).unwrap();
        let err = reopen(&mut conn, "00000000-0000-0000-0000-000000000000").unwrap_err();
        let typed = err.downcast_ref::<PillboxError>().unwrap();
        assert!(matches!(typed, PillboxError::PrescriptionNotFound { .. }));
    }

    /// Fase 7.4 — reopen sobre rx descartada (`deleted_at IS NOT NULL`) debe
    /// devolver `PrescriptionNotFound`. La papelera no se reabre.
    #[test]
    fn reopen_discarded_returns_not_found() {
        let mut conn = open_in_memory(DbScope::Local).unwrap();
        let bottle_id = make_bottle(&mut conn, "reopen-discarded");
        let rx_id = make_rx(&mut conn, &bottle_id, "Descartar y reabrir");

        // Cierra y descarta para garantizar deleted_at IS NOT NULL.
        discard(&mut conn, &rx_id).unwrap();
        // Sanity check: la rx existe y tiene deleted_at != NULL.
        let raw = read_any(&conn, &rx_id).unwrap().unwrap();
        assert!(raw.deleted_at.is_some());

        let err = reopen(&mut conn, &rx_id).unwrap_err();
        let typed = err.downcast_ref::<PillboxError>().unwrap();
        assert!(matches!(typed, PillboxError::PrescriptionNotFound { .. }));

        // El estado no debe haber cambiado.
        let after = read_any(&conn, &rx_id).unwrap().unwrap();
        assert!(after.deleted_at.is_some());
    }

    /// Fase 7.1 — variante explícita: dos `open()` consecutivas en el mismo
    /// bottle devuelven ids distintos y ambas están `ended_at IS NULL`. La
    /// query directa por "abiertas" devuelve 2 filas.
    #[test]
    fn two_opens_same_bottle_both_open_distinct_ids() {
        let mut conn = open_in_memory(DbScope::Local).unwrap();
        let bottle_id = make_bottle(&mut conn, "two-opens");

        let rx_a = open(
            &mut conn,
            &NewPrescription {
                bottle_id: bottle_id.clone(),
                title: "A".into(),
                author_name: None,
                author_email: None,
            },
        )
        .unwrap();
        let rx_b = open(
            &mut conn,
            &NewPrescription {
                bottle_id: bottle_id.clone(),
                title: "B".into(),
                author_name: None,
                author_email: None,
            },
        )
        .unwrap();

        assert_ne!(rx_a.id, rx_b.id);
        assert!(rx_a.ended_at.is_none());
        assert!(rx_b.ended_at.is_none());

        // La query "abiertas en este bottle" (la que usa cmd_prescription_close
        // cuando no se pasa id) devuelve exactamente 2 filas con ids distintos.
        let mut stmt = conn
            .prepare(
                "SELECT id FROM prescriptions
                 WHERE bottle_id = ?1 AND ended_at IS NULL AND deleted_at IS NULL",
            )
            .unwrap();
        let open_ids: Vec<String> = stmt
            .query_map(params![bottle_id], |r| r.get::<_, String>(0))
            .unwrap()
            .collect::<rusqlite::Result<Vec<_>>>()
            .unwrap();
        assert_eq!(open_ids.len(), 2);
        assert!(open_ids.contains(&rx_a.id));
        assert!(open_ids.contains(&rx_b.id));
    }

    #[test]
    fn count_archived_pills_returns_exact_total() {
        use crate::db::store::pills;
        use crate::domain::pill::NewPill;

        let mut conn = open_in_memory(DbScope::Local).unwrap();
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
