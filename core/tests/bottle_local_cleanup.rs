//! Integration tests para `db::cleanup::cleanup_if_last_bottle` y la rama de
//! borrado del .db local en el flujo `bottle delete`.
//!
//! Tres escenarios:
//!
//! - **4.1 (CLI)**: el flujo que ejecuta `cmd_bottle_delete` (unregister +
//!   cleanup_if_last_bottle) debe borrar el `.db` local y sus sidecars
//!   (`-wal`, `-shm`) cuando era el último bottle apuntando a esa ruta. El
//!   wrapper CLI invoca `inquire::Text` interactivo, por lo que no es
//!   testeable como función pura; este test ejercita exactamente las dos
//!   llamadas que la CLI hace tras la confirmación (ver
//!   `pillbox/core/src/cmd/bottle.rs::cmd_bottle_delete`).
//!
//! - **4.3 (global protegida)**: cuando el `db_path` apunta a la DB global,
//!   el guard de `cleanup_if_last_bottle` debe rehusar borrar el fichero
//!   (canonical(path) == canonical(global_db_path) → Ok(false), sin tocar
//!   disco). Para forzar esa igualdad sin asumir nada del HOME del runner,
//!   sobrescribimos HOME al tempdir del test y verificamos que el .db
//!   "global" del tempdir sigue existiendo tras la llamada.
//!
//! - **4.5 (NotFound silencioso)**: si el `.db` ya no existe en disco cuando
//!   se invoca cleanup, debe devolver Ok sin error. Cubre la rama
//!   `ErrorKind::NotFound` de `remove_if_exists`. La rama es compartida por
//!   CLI y HTTP — basta cubrirla en un único test del helper.

use std::path::PathBuf;
use std::sync::Mutex;

use pillbox::db::{cleanup, connection, store::registered_bottles, DbScope};

// Mutex serializa los tests que mutan `HOME` (var de proceso). Sin esto, dos
// tests en paralelo pueden ver un HOME inconsistente y romper la canonicalización
// que hace `cleanup_if_last_bottle` contra `config::global_db_path()`.
static HOME_LOCK: Mutex<()> = Mutex::new(());

/// Helper: crea una DB local (con su schema) en `path`, registra un bottle
/// apuntando a ese path en la DB global, y devuelve el id interno del registro.
fn seed_registered_bottle(global_db: &PathBuf, local_db: &PathBuf) -> i64 {
    // Materializar la DB local (open() ejecuta migrate y crea el fichero).
    {
        let _ = connection::open(local_db, DbScope::Local).expect("open local");
    }
    let global = connection::open(global_db, DbScope::Global).expect("open global");
    registered_bottles::register(
        &global,
        "00000000-0000-0000-0000-000000000001",
        "test",
        "Test",
        &local_db.to_string_lossy(),
    )
    .expect("register");
    let id: i64 = global
        .query_row(
            "SELECT id FROM registered_bottles WHERE db_path = ?1",
            rusqlite::params![local_db.to_string_lossy().as_ref()],
            |r| r.get(0),
        )
        .expect("fetch id");
    id
}

// ─── 4.1 — CLI flow: cleanup borra .db + sidecars ────────────────────────────

#[test]
fn cli_delete_last_bottle_removes_db_and_sidecars() {
    let tmp = tempfile::tempdir().unwrap();
    let global_db = tmp.path().join("global.db");
    let local_db = tmp.path().join("local.db");

    let reg_id = seed_registered_bottle(&global_db, &local_db);

    // Forzar la existencia de sidecars -wal / -shm para verificar que también
    // se borran. Un INSERT en modo WAL típicamente los crea, pero los hacemos
    // explícitos para no depender del checkpoint.
    let wal = local_db.with_file_name("local.db-wal");
    let shm = local_db.with_file_name("local.db-shm");
    std::fs::write(&wal, b"x").unwrap();
    std::fs::write(&shm, b"x").unwrap();

    let global = connection::open(&global_db, DbScope::Global).unwrap();

    // Replica exacta del par de llamadas que hace `cmd_bottle_delete` tras la
    // confirmación del slug por parte del usuario.
    assert!(registered_bottles::unregister(&global, reg_id).unwrap());
    let removed = cleanup::cleanup_if_last_bottle(&global, &local_db).unwrap();

    assert!(removed, "cleanup should report file removal");
    assert!(!local_db.exists(), ".db should be gone");
    assert!(!wal.exists(), "-wal sidecar should be gone");
    assert!(!shm.exists(), "-shm sidecar should be gone");
}

// ─── 4.3 — Global DB protegida por el guard ──────────────────────────────────

#[test]
fn cleanup_refuses_to_delete_global_db() {
    let _guard = HOME_LOCK.lock().unwrap_or_else(|e| e.into_inner());

    let tmp = tempfile::tempdir().unwrap();
    // Sobrescribir HOME → config::global_db_path() apuntará dentro del tempdir.
    let prev_home = std::env::var_os("HOME");
    std::env::set_var("HOME", tmp.path());

    let global_db = pillbox::config::global_db_path();
    std::fs::create_dir_all(global_db.parent().unwrap()).unwrap();

    // Materializar la DB global en su ubicación canónica del tempdir.
    let global = connection::open(&global_db, DbScope::Global).expect("open global");

    // Insertar un registro apuntando a la propia DB global. Aunque tras
    // unregister no queden filas, el guard de locality debe prevalecer.
    registered_bottles::register(
        &global,
        "00000000-0000-0000-0000-0000000000ff",
        "global-bottle",
        "Global Bottle",
        &global_db.to_string_lossy(),
    )
    .unwrap();
    let reg_id: i64 = global
        .query_row(
            "SELECT id FROM registered_bottles WHERE db_path = ?1",
            rusqlite::params![global_db.to_string_lossy().as_ref()],
            |r| r.get(0),
        )
        .unwrap();
    registered_bottles::unregister(&global, reg_id).unwrap();

    // Ahora registered_bottles está vacío. Sin el guard, cleanup borraría
    // la DB global.
    let removed = cleanup::cleanup_if_last_bottle(&global, &global_db).unwrap();

    assert!(!removed, "cleanup must refuse to remove the global DB");
    assert!(
        global_db.exists(),
        "global DB file must remain on disk after cleanup attempt"
    );

    // Restaurar HOME para no contaminar otros tests del mismo binario.
    drop(global);
    match prev_home {
        Some(v) => std::env::set_var("HOME", v),
        None => std::env::remove_var("HOME"),
    }
}

// ─── 4.5 — NotFound silencioso ───────────────────────────────────────────────

#[test]
fn cleanup_silences_notfound_when_db_already_missing() {
    let tmp = tempfile::tempdir().unwrap();
    let global_db = tmp.path().join("global.db");
    let local_db = tmp.path().join("local.db");

    let reg_id = seed_registered_bottle(&global_db, &local_db);

    // Borrar a mano el .db (y sidecars si existen) antes del cleanup.
    std::fs::remove_file(&local_db).unwrap();
    let _ = std::fs::remove_file(local_db.with_file_name("local.db-wal"));
    let _ = std::fs::remove_file(local_db.with_file_name("local.db-shm"));
    assert!(!local_db.exists());

    let global = connection::open(&global_db, DbScope::Global).unwrap();
    assert!(registered_bottles::unregister(&global, reg_id).unwrap());

    // Tras unregister, count == 0 y el fichero no existe → debe devolver
    // Ok(true) silenciando todos los NotFound de `.db`, `-wal`, `-shm`.
    let removed = cleanup::cleanup_if_last_bottle(&global, &local_db).unwrap();
    assert!(
        removed,
        "cleanup should report removal (count == 0 → attempted, NotFound silenced)"
    );
    assert!(!local_db.exists());
}
