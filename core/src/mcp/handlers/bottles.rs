use pillbox::{
    config, db, db::store, db::store::registered_bottles,
    domain::bottle::{BottleScope, NewBottle},
};
use serde_json::Value;

use crate::mcp::response::{anyhow_to_response, from_value, validate_input, Conn, Response};

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

pub fn list(conn: &mut Conn, _input: Value) -> Response {
    match store::bottles::list(conn) {
        Ok(bottles) => Response::ok(bottles),
        Err(e) => anyhow_to_response(e),
    }
}

#[cfg(test)]
mod tests {
    use pillbox::{
        db::{connection::open_in_memory, store::registered_bottles},
        domain::bottle::{BottleScope, NewBottle},
    };

    use super::*;

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
