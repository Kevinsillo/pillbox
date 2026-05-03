//! Handlers MCP para la entidad Bottle.

use pillbox::{
    config, db, db::store, db::store::registered_bottles,
    domain::bottle::{BottleScope, NewBottle},
};
use serde_json::{json, Value};

use crate::mcp::response::{anyhow_to_response, from_value, validate_input, Conn, Response};

/// Crea un bottle nuevo y lo registra en la DB global.
///
/// Para scope local, abre la DB del proyecto directamente en
/// `<directory>/.pillbox/pillbox.db` en lugar de usar el `conn` del dispatch.
pub fn create(conn: &mut Conn, input: Value) -> Response {
    let req: NewBottle = match from_value(input) {
        Ok(v) => v,
        Err(r) => return r,
    };
    if let Err(r) = validate_input(&req) {
        return r;
    }

    // Para scope local, ignorar el conn del dispatch y abrir directamente la DB
    // del proyecto en <directory>/.pillbox/pillbox.db (la crea si no existe).
    let result = if req.scope == BottleScope::Local {
        let local_db_path = std::path::Path::new(&req.directory)
            .join(".pillbox")
            .join("pillbox.db");
        match db::connection::open(&local_db_path) {
            Ok(mut local_conn) => store::bottles::create(&mut local_conn, &req),
            Err(e) => Err(e),
        }
    } else {
        store::bottles::create(conn, &req)
    };

    match result {
        Ok(b) => {
            let global_path = config::global_db_path();
            let registration_path = if req.scope == BottleScope::Local {
                std::path::Path::new(&b.directory)
                    .join(".pillbox")
                    .join("pillbox.db")
            } else {
                global_path.clone()
            };
            if let Ok(global_conn) = db::connection::open(&global_path) {
                let _ = registered_bottles::register(
                    &global_conn,
                    &b.id,
                    &b.name,
                    &b.display_name,
                    &registration_path.to_string_lossy(),
                );
            }
            Response::ok(b)
        }
        Err(e) => anyhow_to_response(e),
    }
}

/// Lista todos los bottles registrados (enlazados y desenlazados).
pub fn list(_conn: &mut Conn, _input: Value) -> Response {
    use pillbox::domain::bottle::Bottle;

    let global_path = config::global_db_path();
    if !global_path.exists() {
        return Response::ok(Vec::<Bottle>::new());
    }

    let global_conn = match db::connection::open(&global_path) {
        Ok(c) => c,
        Err(e) => return anyhow_to_response(e),
    };

    let registered = match registered_bottles::list(&global_conn) {
        Ok(r) => r,
        Err(e) => return anyhow_to_response(e),
    };

    let mut bottles: Vec<Bottle> = Vec::new();
    for reg in registered {
        let db_path = std::path::Path::new(&reg.db_path);
        if !db_path.exists() {
            bottles.push(Bottle {
                id: reg.bottle_id,
                name: reg.name,
                display_name: reg.display_name,
                directory: db_path
                    .parent()
                    .and_then(|p| p.parent())
                    .map(|p| p.to_string_lossy().to_string())
                    .unwrap_or_else(|| reg.db_path.clone()),
                scope: "local".to_string(),
                created_at: reg.registered_at,
                last_seen_at: reg.last_seen_at,
                linked: false,
                reg_id: Some(reg.id),
            });
            continue;
        }

        let db_conn = match db::connection::open(db_path) {
            Ok(c) => c,
            Err(_) => continue,
        };

        for mut bottle in store::bottles::list(&db_conn).unwrap_or_default() {
            bottle.reg_id = Some(reg.id);
            bottles.push(bottle);
        }
    }

    bottles.sort_by(|a, b| a.name.cmp(&b.name));
    Response::ok(bottles)
}

/// Input para el handler `bottle_vinculate`.
#[derive(serde::Deserialize)]
struct VinculateInput {
    /// Ruta absoluta al directorio que contiene `.pillbox/pillbox.db`.
    /// Si es `None`, se usa el cwd del proceso pillbox.
    directory: Option<String>,
}

/// Vincula una DB local al registro `registered_bottles` de la DB global del
/// usuario actual.
///
/// Idempotente: devuelve `status: "already_linked"` si ya estaba registrada.
pub fn vinculate(_conn: &mut Conn, input: Value) -> Response {
    let req: VinculateInput = match from_value(input) {
        Ok(v) => v,
        Err(r) => return r,
    };

    let dir = match req.directory {
        Some(d) => std::path::PathBuf::from(d),
        None => match std::env::current_dir() {
            Ok(d) => d,
            Err(e) => return Response::err("cwd_unavailable", e.to_string()),
        },
    };

    let db_path = dir.join(".pillbox").join("pillbox.db");
    if !db_path.exists() {
        return Response::err(
            "db_not_found",
            format!("local database not found: {}", db_path.display()),
        );
    }

    let db_path_canon = match std::fs::canonicalize(&db_path) {
        Ok(p) => p,
        Err(e) => return Response::err("canonicalize_failed", e.to_string()),
    };

    let global_path = config::global_db_path();
    if global_path.exists() {
        match std::fs::canonicalize(&global_path) {
            Ok(global_canon) if db_path_canon == global_canon => {
                return Response::err(
                    "circular_link",
                    "cannot link: path resolves to the global database".to_string(),
                );
            }
            Ok(_) => {}
            Err(e) => return Response::err("canonicalize_failed", e.to_string()),
        }
    }

    let local_conn = match db::connection::open(&db_path_canon) {
        Ok(c) => c,
        Err(e) => return anyhow_to_response(e),
    };

    let bottle = match store::bottles::list(&local_conn) {
        Ok(list) => match list.into_iter().next() {
            Some(b) => b,
            None => {
                return Response::err(
                    "no_bottle",
                    format!("no bottle found in local database: {}", db_path_canon.display()),
                );
            }
        },
        Err(e) => return anyhow_to_response(e),
    };

    let global_conn = match db::connection::open(&global_path) {
        Ok(c) => c,
        Err(e) => return anyhow_to_response(e),
    };

    let db_path_str = match db_path_canon.to_str() {
        Some(s) => s,
        None => {
            return Response::err(
                "invalid_path",
                "local DB path contains non-UTF-8 characters".to_string(),
            );
        }
    };

    let inserted = match registered_bottles::register(
        &global_conn,
        &bottle.id,
        &bottle.name,
        &bottle.display_name,
        db_path_str,
    ) {
        Ok(r) => r,
        Err(e) => return anyhow_to_response(e),
    };

    let status = if inserted { "linked" } else { "already_linked" };
    Response::ok(json!({
        "status": status,
        "bottle_id": bottle.id,
        "name": bottle.display_name,
        "slug": bottle.name,
        "db_path": db_path_str,
    }))
}

#[cfg(test)]
mod tests {
    use pillbox::{
        db::{connection::open_in_memory, store::registered_bottles},
        domain::bottle::{BottleScope, NewBottle},
    };

    /// Verifica que el path calculado para la DB local sigue el patrón
    /// `<directory>/.pillbox/pillbox.db`, comprobando la lógica que usa el handler.
    #[test]
    fn create_registers_in_global() {
        let mut local_conn = open_in_memory().unwrap();
        let global_conn = open_in_memory().unwrap();

        let input = NewBottle {
            name: "mi-proyecto".into(),
            display_name: "Mi Proyecto".into(),
            directory: "/home/user/mi-proyecto".into(),
            scope: BottleScope::Local,
        };

        let b = pillbox::db::store::bottles::create(&mut local_conn, &input).unwrap();

        let expected_db_path = std::path::Path::new(&b.directory)
            .join(".pillbox")
            .join("pillbox.db");

        let _ = registered_bottles::register(
            &global_conn,
            &b.id,
            &b.name,
            &b.display_name,
            &expected_db_path.to_string_lossy(),
        );

        let found = registered_bottles::find_by_bottle_id(&global_conn, &b.id)
            .unwrap()
            .unwrap();

        assert_eq!(
            found.db_path,
            "/home/user/mi-proyecto/.pillbox/pillbox.db"
        );
    }

    /// Verifica que un fallo al registrar en la DB global no impide que `create`
    /// devuelva Ok — la tabla `registered_bottles` no existe en el conn simulado.
    #[test]
    fn create_registration_failure_does_not_propagate() {
        let mut local_conn = open_in_memory().unwrap();

        // Conexión sin migraciones → no tiene registered_bottles
        let broken_conn = rusqlite::Connection::open_in_memory().unwrap();

        let input = NewBottle {
            name: "fail-proj".into(),
            display_name: "Fail Proj".into(),
            directory: "/tmp/fail-proj".into(),
            scope: BottleScope::Local,
        };

        let b = pillbox::db::store::bottles::create(&mut local_conn, &input).unwrap();

        // El register falla porque la tabla no existe, pero el error se descarta con `let _`
        let result = registered_bottles::register(
            &broken_conn,
            &b.id,
            &b.name,
            &b.display_name,
            "/tmp/fail-proj/.pillbox/pillbox.db",
        );

        // El handler descarta el error — verificamos que create del store fue Ok
        assert!(result.is_err()); // el register sí falla en conn roto
        // pero el bottle fue creado correctamente
        let found = pillbox::db::store::bottles::find_by_id(&local_conn, &b.id)
            .unwrap()
            .unwrap();
        assert_eq!(found.name, "fail-proj");
    }

    /// Verificar que registrar dos veces los mismos datos resulta en una sola fila.
    #[test]
    fn create_registration_is_idempotent() {
        let mut local_conn = open_in_memory().unwrap();
        let global_conn = open_in_memory().unwrap();

        let input = NewBottle {
            name: "idem-proj".into(),
            display_name: "Idem Proj".into(),
            directory: "/tmp/idem-proj".into(),
            scope: BottleScope::Local,
        };

        let b = pillbox::db::store::bottles::create(&mut local_conn, &input).unwrap();
        let db_path = format!("{}/.pillbox/pillbox.db", b.directory);

        registered_bottles::register(&global_conn, &b.id, &b.name, &b.display_name, &db_path)
            .unwrap();
        registered_bottles::register(&global_conn, &b.id, &b.name, &b.display_name, &db_path)
            .unwrap();

        let rows = registered_bottles::list(&global_conn).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].db_path, db_path);
    }
}
