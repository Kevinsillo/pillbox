//! Integration tests del loop persistente `pillbox mcp run`.
//!
//! Cubre:
//!   - 3.14 → multiplexación: 100 requests con UUIDs distintos, las
//!     respuestas pueden llegar fuera de orden pero los `id` deben casar.
//!   - 3.15 → EOF: cerrar stdin del child hace que el proceso exite con
//!     código 0 en menos de 1 segundo (no quedan recursos colgados).
//!   - 3.19 → pureza de stdout: cada línea de stdout es JSON parseable
//!     incluso cuando una operación dispara un WARN del tracing
//!     (tracing va a stderr, stdout queda limpio).
//!
//! Cada test arranca el binario con HOME apuntando a un tempdir aislado y
//! con una DB global pre-sembrada — el subcomando no la crea automágicamente.

use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use serde_json::{json, Value};

/// Handle al subproceso `pillbox mcp run` para tests.
///
/// NO implementa `Drop` deliberadamente: los tests destructuran el struct
/// para mover stdin/stdout a hilos o cerrarlos manualmente, y `Drop` sobre
/// un struct con campos parcialmente movidos no compila. Cada test es
/// responsable de invocar `child.kill()` / `child.wait()` al final.
struct McpProc {
    child: Child,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
    cwd: PathBuf,
    _tmp: tempfile::TempDir,
}

fn spawn_mcp() -> McpProc {
    let tmp = tempfile::tempdir().unwrap();
    let global_db = tmp.path().join(".pillbox").join("pillbox.db");
    std::fs::create_dir_all(global_db.parent().unwrap()).unwrap();
    {
        let _conn = pillbox::db::connection::open(&global_db, pillbox::db::DbScope::Global)
            .expect("seed global DB");
    }

    let exe = env!("CARGO_BIN_EXE_pillbox");
    let mut child = Command::new(exe)
        .args(["mcp", "run"])
        .env("HOME", tmp.path())
        .current_dir(tmp.path())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn pillbox mcp run");

    let stdin = child.stdin.take().expect("stdin");
    let stdout = BufReader::new(child.stdout.take().expect("stdout"));
    let cwd = tmp.path().to_path_buf();
    McpProc {
        child,
        stdin,
        stdout,
        cwd,
        _tmp: tmp,
    }
}

#[test]
fn mcp_loop_correlates_100_concurrent_requests() {
    let proc = spawn_mcp();
    let cwd_str = proc.cwd.to_string_lossy().to_string();
    let McpProc {
        mut child,
        stdin,
        stdout: mut stdout_reader,
        cwd: _,
        _tmp,
    } = proc;

    // Pre-generamos 100 ids únicos. El tool elegido es `bottle_list` — no
    // requiere prescription abierta y devuelve una lista (puede ser vacía).
    let ids: Vec<String> = (0..100).map(|i| format!("req-{:04}-uuid", i)).collect();

    // Escritor en un hilo aparte para no serializar con la lectura.
    let stdin_ids = ids.clone();
    let cwd_for_writer = cwd_str.clone();
    let mut stdin = stdin;
    let writer = thread::spawn(move || {
        for id in &stdin_ids {
            let payload = json!({
                "id": id,
                "tool": "bottle_list",
                "input": {},
                "cwd": cwd_for_writer,
            });
            writeln!(stdin, "{}", payload).expect("write request");
        }
        stdin.flush().unwrap();
        stdin // devolverlo para que viva más que el lector
    });

    let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
    let deadline = Instant::now() + Duration::from_secs(20);

    while seen.len() < ids.len() {
        if Instant::now() > deadline {
            panic!(
                "timeout: only {}/{} responses received",
                seen.len(),
                ids.len()
            );
        }
        let mut line = String::new();
        let n = stdout_reader.read_line(&mut line).expect("read_line");
        if n == 0 {
            panic!(
                "stdout EOF before all responses arrived ({}/{})",
                seen.len(),
                ids.len()
            );
        }
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let v: Value = serde_json::from_str(trimmed)
            .unwrap_or_else(|e| panic!("non-JSON line in stdout: {} ({})", trimmed, e));
        let id = v["id"].as_str().expect("response.id is string").to_string();
        assert!(
            ids.contains(&id),
            "response id {:?} does not match any sent request",
            id
        );
        assert!(seen.insert(id), "duplicate response id");
    }

    let _ = writer.join();
    let _ = child.kill();
    let _ = child.wait();
}

#[test]
fn mcp_loop_exits_cleanly_on_eof() {
    let proc = spawn_mcp();
    // Pequeña pausa para que el child se asiente.
    thread::sleep(Duration::from_millis(100));

    // Desestructuramos para mover stdin fuera (drop = cerrar EOF) sin que
    // `Drop for McpProc` se ejecute (lo que mataría el child antes de
    // poder observar su salida limpia).
    let McpProc {
        mut child,
        stdin,
        stdout: _stdout,
        cwd: _cwd,
        _tmp,
    } = proc;
    drop(stdin); // EOF al child

    let start = Instant::now();
    loop {
        if let Some(status) = child.try_wait().expect("try_wait") {
            let elapsed = start.elapsed();
            assert!(
                status.success(),
                "process exited with non-zero status: {:?}",
                status
            );
            assert!(
                elapsed < Duration::from_secs(1),
                "process took {:?} to exit (limit 1s)",
                elapsed
            );
            return;
        }
        if start.elapsed() > Duration::from_secs(2) {
            let _ = child.kill();
            panic!("process did not exit within 2s after EOF");
        }
        thread::sleep(Duration::from_millis(20));
    }
}

#[test]
fn mcp_loop_stdout_is_pure_ndjson_even_with_warnings() {
    // Envío de varias requests que potencialmente disparan log lines:
    //   - JSON inválido → parse_error (puede registrar nada o un WARN)
    //   - cwd inválido → missing_cwd
    //   - bottle_id inexistente en un read → respuesta de error
    // Lo importante: cada línea que llega por stdout DEBE ser JSON válido.
    let proc = spawn_mcp();
    let cwd_str = proc.cwd.to_string_lossy().to_string();
    let McpProc {
        mut child,
        mut stdin,
        stdout: mut stdout_reader,
        cwd: _,
        _tmp,
    } = proc;
    // 1) JSON malformado
    writeln!(stdin, "{{not valid json").unwrap();
    // 2) cwd vacío
    writeln!(
        stdin,
        r#"{{"id":"a","tool":"bottle_list","input":{{}},"cwd":""}}"#
    )
    .unwrap();
    // 3) Request válida
    writeln!(
        stdin,
        r#"{{"id":"b","tool":"bottle_list","input":{{}},"cwd":{}}}"#,
        serde_json::to_string(&cwd_str).unwrap()
    )
    .unwrap();
    // 4) Tool desconocido
    writeln!(
        stdin,
        r#"{{"id":"c","tool":"definitely_not_a_tool","input":{{}},"cwd":{}}}"#,
        serde_json::to_string(&cwd_str).unwrap()
    )
    .unwrap();
    stdin.flush().unwrap();
    drop(stdin); // EOF → child saldrá tras procesar

    let mut count = 0;
    loop {
        let mut line = String::new();
        let n = stdout_reader.read_line(&mut line).expect("read_line");
        if n == 0 {
            break; // EOF
        }
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let v: Value = serde_json::from_str(trimmed)
            .unwrap_or_else(|e| panic!("stdout line is not JSON: {:?} ({})", trimmed, e));
        // Cada respuesta debe ser un objeto con campo "ok".
        assert!(v.get("ok").is_some(), "response missing 'ok': {}", trimmed);
        count += 1;
    }
    assert!(count >= 3, "expected at least 3 responses, got {}", count);
    let _ = child.wait();
}
