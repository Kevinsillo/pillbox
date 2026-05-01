//! Subcomandos `pillbox serve *`.

use anyhow::Result;
use owo_colors::OwoColorize;
use rust_i18n::t;

use crate::output;

/// Arranca el servidor HTTP en el puerto dado, en modo daemon o inline.
pub async fn cmd_serve_start(port: u16, inline: bool) -> Result<()> {
    if inline {
        cmd_serve_run(port).await
    } else {
        cmd_serve_daemon(port).await
    }
}

/// Lanza el servidor en un proceso hijo desvinculado del terminal.
///
/// Escribe el PID en el fichero de PID y espera 300 ms para verificar
/// que el proceso arrancó correctamente.
async fn cmd_serve_daemon(port: u16) -> Result<()> {
    let pid_path = pillbox::config::pid_path();

    if let Some(pid) = read_pid(&pid_path) {
        if process_alive(pid) {
            anyhow::bail!("{}", t!("serve.error.already_running", pid = pid));
        }
        let _ = std::fs::remove_file(&pid_path);
    }

    pillbox::config::resolve_db_path()
        .ok_or_else(|| anyhow::anyhow!("{}", t!("serve.error.no_db")))?;

    let exe = std::env::current_exe()?;
    let log_path = pillbox::config::log_path();
    let log_file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_path)?;
    let child = std::process::Command::new(exe)
        .args(["serve", "start", "--port", &port.to_string(), "--inline"])
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(log_file)
        .spawn()?;
    let pid = child.id();
    std::fs::write(&pid_path, pid.to_string())?;
    std::thread::sleep(std::time::Duration::from_millis(300));
    if !process_alive(pid) {
        let _ = std::fs::remove_file(&pid_path);
        let hint = std::fs::read_to_string(&log_path)
            .ok()
            .and_then(|s| s.lines().last().map(|l| l.to_string()))
            .unwrap_or_default();
        anyhow::bail!("{}\n{}", t!("serve.error.start_failed"), hint);
    }
    println!(
        "\n{} {}\n  {}\n",
        "✓".green().bold(),
        t!("serve.daemon_started", pid = pid, port = port),
        format!("http://localhost:{}", port).cyan()
    );
    Ok(())
}

/// Ejecuta el servidor en primer plano (modo inline), bloqueando hasta CTRL+C.
async fn cmd_serve_run(port: u16) -> Result<()> {
    let pid_path = pillbox::config::pid_path();
    let db_path = pillbox::config::resolve_db_path()
        .ok_or_else(|| anyhow::anyhow!("{}", t!("serve.error.no_db")))?;
    let global_db_path = pillbox::config::global_db_path();
    std::fs::write(&pid_path, std::process::id().to_string())?;
    let result = crate::server::run(port, db_path, global_db_path).await;
    let _ = std::fs::remove_file(&pid_path);
    result
}

/// Para el servidor daemon enviando SIGTERM al proceso y eliminando el fichero de PID.
pub fn cmd_serve_stop() -> Result<()> {
    let pid_path = pillbox::config::pid_path();
    let pid = read_pid(&pid_path).ok_or_else(|| anyhow::anyhow!("{}", t!("serve.stop.none")))?;

    if !process_alive(pid) {
        let _ = std::fs::remove_file(&pid_path);
        anyhow::bail!("{}", t!("serve.stop.dead", pid = pid));
    }

    #[cfg(unix)]
    {
        use std::process::Command;
        Command::new("kill").arg(pid.to_string()).status()?;
    }

    let _ = std::fs::remove_file(&pid_path);
    println!(
        "\n{} {}\n",
        "✓".green().bold(),
        t!("serve.stop.done", pid = pid)
    );
    Ok(())
}

/// Muestra si el servidor está escuchando en el puerto por defecto.
pub fn cmd_serve_status() -> Result<()> {
    let pid_path = pillbox::config::pid_path();
    let port = pillbox::config::DEFAULT_PORT;
    let pid = read_pid(&pid_path);
    let running = server_listening(port);
    output::fmt::serve_status(running, pid, port);
    Ok(())
}

/// Lee el PID almacenado en un fichero. Devuelve `None` si el fichero no existe o no es válido.
pub fn read_pid(path: &std::path::Path) -> Option<u32> {
    std::fs::read_to_string(path).ok()?.trim().parse().ok()
}

/// Verifica si hay algo escuchando en `localhost:{port}` con un timeout de 200 ms.
pub fn server_listening(port: u16) -> bool {
    use std::net::TcpStream;
    use std::time::Duration;
    TcpStream::connect_timeout(
        &std::net::SocketAddr::from(([127, 0, 0, 1], port)),
        Duration::from_millis(200),
    )
    .is_ok()
}

/// Comprueba si un proceso sigue vivo.
///
/// En Linux usa `/proc/{pid}`, en otros Unix envía señal 0 con `kill`.
fn process_alive(pid: u32) -> bool {
    #[cfg(target_os = "linux")]
    {
        std::path::Path::new(&format!("/proc/{}", pid)).exists()
    }
    #[cfg(all(unix, not(target_os = "linux")))]
    {
        std::process::Command::new("kill")
            .args(["-0", &pid.to_string()])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    }
    #[cfg(not(unix))]
    {
        false
    }
}
