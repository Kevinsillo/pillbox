//! Subcomandos `pillbox serve *`.
//!
//! Gestiona el ciclo de vida del servidor HTTP como **servicio del sistema**
//! (systemd user-unit en Linux, LaunchAgent en macOS, Service Control Manager
//! en Windows). Las operaciones expuestas son `install`, `uninstall`,
//! `start`, `stop`, `status` e `info`. La variante `run` (oculta) es la que
//! ejecuta realmente el servidor en primer plano y es invocada por el
//! gestor de servicios.

use std::ffi::OsString;
use std::fs;
use std::path::PathBuf;

use anyhow::Result;
use indoc::formatdoc;
use owo_colors::OwoColorize;
use rust_i18n::t;
use service_manager::{
    ServiceInstallCtx, ServiceLabel, ServiceManager, ServiceStartCtx, ServiceStopCtx,
    ServiceUninstallCtx,
};

use pillbox::config::{serve_port_path, DEFAULT_PORT};

// ─── Constantes ───────────────────────────────────────────────────────────────

/// Etiqueta única del servicio en formato qualifier.organization.application.
/// Usada por systemd, launchd y el SC manager para identificar el servicio.
const SERVICE_LABEL: &str = "pillbox-daemon";

/// Marcador insertado en `/etc/hosts` para identificar la entrada de pillbox.
const HOSTS_MARKER: &str = "# pillbox-serve";

/// Hostname reservado al que apunta la entrada en `hosts` (siempre 127.0.0.1).
const HOSTS_HOSTNAME: &str = "pillbox.local";

// ─── Helpers de servicio ──────────────────────────────────────────────────────

/// Construye el `ServiceLabel` parseando `SERVICE_LABEL`.
fn service_label() -> ServiceLabel {
    SERVICE_LABEL
        .parse()
        .expect("SERVICE_LABEL must be a valid service label")
}

/// Crea el `ServiceManager` apropiado para la plataforma actual.
///
/// - Linux/macOS: gestor nativo a nivel de **usuario** (systemd user unit / LaunchAgent).
/// - Windows: gestor nativo a nivel de **sistema** (sc).
fn build_service_manager() -> Result<Box<dyn ServiceManager>> {
    let mut manager = <dyn ServiceManager>::native()
        .map_err(|e| anyhow::anyhow!("failed to create service manager: {}", e))?;

    #[cfg(any(target_os = "linux", target_os = "macos"))]
    {
        manager
            .set_level(service_manager::ServiceLevel::User)
            .map_err(|e| anyhow::anyhow!("failed to set user-level service: {}", e))?;
    }

    Ok(manager)
}

/// Detecta si el servicio está instalado en el sistema.
///
/// Estrategia por plataforma:
/// - Linux: existe `~/.config/systemd/user/pillbox-daemon.service`.
/// - macOS: existe `~/Library/LaunchAgents/pillbox-daemon.plist`.
/// - Windows: `sc query pillbox-daemon` retorna éxito.
pub(crate) fn is_installed() -> bool {
    #[cfg(target_os = "linux")]
    {
        dirs::home_dir()
            .map(|h| {
                h.join(".config")
                    .join("systemd")
                    .join("user")
                    .join(format!("{}.service", SERVICE_LABEL))
            })
            .map(|p| p.exists())
            .unwrap_or(false)
    }

    #[cfg(target_os = "macos")]
    {
        dirs::home_dir()
            .map(|h| {
                h.join("Library")
                    .join("LaunchAgents")
                    .join(format!("{}.plist", SERVICE_LABEL))
            })
            .map(|p| p.exists())
            .unwrap_or(false)
    }

    #[cfg(target_os = "windows")]
    {
        use std::process::{Command, Stdio};
        Command::new("sc")
            .args(["query", SERVICE_LABEL])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    }

    #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
    {
        false
    }
}

// ─── Helpers de hosts file ────────────────────────────────────────────────────

/// Ruta del fichero `hosts` por plataforma.
fn hosts_path() -> PathBuf {
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    {
        PathBuf::from("/etc/hosts")
    }
    #[cfg(target_os = "windows")]
    {
        PathBuf::from(r"C:\Windows\System32\drivers\etc\hosts")
    }
    #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
    {
        PathBuf::from("/etc/hosts")
    }
}

/// Añade idempotentemente la entrada `127.0.0.1 pillbox.local # pillbox-serve`
/// al fichero `hosts`. Si la línea ya existe (detectada por el marcador), no
/// hace nada.
///
/// Devuelve error si no tiene permisos de escritura — el caller debe tratarlo
/// como **no fatal** (instalación continúa, sólo se pierde el alias DNS).
fn write_hosts_entry() -> std::io::Result<()> {
    let path = hosts_path();
    let current = fs::read_to_string(&path).unwrap_or_default();

    if current.lines().any(|l| l.contains(HOSTS_MARKER)) {
        return Ok(());
    }

    let needs_newline = !current.is_empty() && !current.ends_with('\n');
    let mut new_content = current;
    if needs_newline {
        new_content.push('\n');
    }
    new_content.push_str(&format!(
        "127.0.0.1\t{}\t{}\n",
        HOSTS_HOSTNAME, HOSTS_MARKER
    ));

    fs::write(&path, new_content)
}

/// Elimina del fichero `hosts` cualquier línea que contenga el marcador
/// `# pillbox-serve`. Operación idempotente: si no existe, no hace nada.
fn remove_hosts_entry() -> std::io::Result<()> {
    let path = hosts_path();
    let current = match fs::read_to_string(&path) {
        Ok(s) => s,
        Err(_) => return Ok(()),
    };

    let mut changed = false;
    let new_content: String = current
        .lines()
        .filter(|l| {
            let keep = !l.contains(HOSTS_MARKER);
            if !keep {
                changed = true;
            }
            keep
        })
        .map(|l| format!("{}\n", l))
        .collect();

    if !changed {
        return Ok(());
    }

    fs::write(&path, new_content)
}

/// Comprueba si la entrada de pillbox existe en `hosts`. Útil para decidir
/// la URL canónica que se muestra al usuario.
fn hosts_entry_present() -> bool {
    let path = hosts_path();
    fs::read_to_string(&path)
        .map(|s| s.lines().any(|l| l.contains(HOSTS_MARKER)))
        .unwrap_or(false)
}

// ─── Helpers de serve.port ────────────────────────────────────────────────────

/// Lee el puerto persistido en `~/.pillbox/serve.port`. Devuelve `None` si
/// el fichero no existe o no contiene un `u16` válido.
fn read_serve_port() -> Option<u16> {
    let path = serve_port_path();
    fs::read_to_string(&path).ok()?.trim().parse().ok()
}

/// Persiste el puerto activo en `~/.pillbox/serve.port`. Crea el directorio
/// padre si fuese necesario.
fn write_serve_port(port: u16) -> std::io::Result<()> {
    let path = serve_port_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&path, port.to_string())
}

// ─── Subcomandos ──────────────────────────────────────────────────────────────

/// Ejecuta el servidor HTTP en primer plano en el puerto dado.
///
/// Esta variante (oculta en la CLI) es la que invoca realmente el gestor de
/// servicios mediante `pillbox serve run --port N`. Bloquea hasta recibir
/// SIGTERM o CTRL+C.
pub async fn cmd_serve_run(port: u16) -> Result<()> {
    // Fase 1: serve usa el cwd del proceso. Fase 2 introducirá un pool
    // cuyo cwd llegará desde la request (X-Pillbox-Cwd) y llamará a
    // `resolve_db_path(&req_cwd)` directamente.
    let db_path = pillbox::config::resolve_db_path_from_env()
        .ok_or_else(|| anyhow::anyhow!("{}", t!("serve.error.no_db")))?;
    let global_db_path = pillbox::config::global_db_path();
    pillbox::server::run(port, db_path, global_db_path).await
}

/// Instala el servidor HTTP como servicio del sistema.
///
/// Pasos:
/// 1. Registra el servicio con `pillbox serve run --port N`.
/// 2. Persiste el puerto en `~/.pillbox/serve.port`.
/// 3. Intenta añadir la entrada al fichero `hosts` (no fatal si falla).
pub fn cmd_serve_install(port: u16) -> Result<()> {
    if is_installed() {
        anyhow::bail!("{}", t!("serve.error.already_installed"));
    }
    let manager = build_service_manager()?;
    let exe = std::env::current_exe()?;
    let label = service_label();

    #[cfg(target_os = "linux")]
    let service_contents = Some(formatdoc!(
        "
        [Unit]
        Description=pillbox-daemon
        After=network.target

        [Service]
        ExecStart={} serve run --port {}
        Restart=on-failure

        [Install]
        WantedBy=default.target
        ",
        exe.display(),
        port
    ));
    #[cfg(not(target_os = "linux"))]
    let service_contents = None;

    let ctx = ServiceInstallCtx {
        label: label.clone(),
        program: exe,
        args: vec![
            OsString::from("serve"),
            OsString::from("run"),
            OsString::from("--port"),
            OsString::from(port.to_string()),
        ],
        contents: service_contents,
        username: None,
        working_directory: None,
        environment: None,
        autostart: true,
    };

    manager
        .install(ctx)
        .map_err(|e| anyhow::anyhow!("failed to install service: {}", e))?;

    if let Err(e) = write_serve_port(port) {
        tracing::warn!("failed to write serve.port: {}", e);
    }

    if let Err(e) = write_hosts_entry() {
        tracing::warn!("hosts write failed: {}", e);
        eprintln!("{} {}", "!".yellow().bold(), t!("serve.error.hosts_write"));
    }

    println!("\n{} {}\n", "✓".green().bold(), t!("serve.install.success"));
    Ok(())
}

/// Desinstala el servicio del sistema.
///
/// Pasos:
/// 1. Elimina la entrada de `hosts` (no fatal).
/// 2. Llama a `ServiceManager::uninstall` (error si no estaba instalado).
/// 3. Borra `~/.pillbox/serve.port`.
pub fn cmd_serve_uninstall() -> Result<()> {
    if !is_installed() {
        anyhow::bail!("{}", t!("serve.error.not_installed"));
    }

    if let Err(e) = remove_hosts_entry() {
        tracing::warn!("hosts remove failed: {}", e);
    }

    let manager = build_service_manager()?;
    manager
        .uninstall(ServiceUninstallCtx {
            label: service_label(),
        })
        .map_err(|e| anyhow::anyhow!("failed to uninstall service: {}", e))?;

    let _ = fs::remove_file(serve_port_path());

    println!(
        "\n{} {}\n",
        "✓".green().bold(),
        t!("serve.uninstall.success")
    );
    Ok(())
}

/// Arranca el servicio (delega en el gestor de servicios).
pub fn cmd_serve_start() -> Result<()> {
    if !is_installed() {
        anyhow::bail!("{}", t!("serve.error.not_installed"));
    }
    let manager = build_service_manager()?;
    manager
        .start(ServiceStartCtx {
            label: service_label(),
        })
        .map_err(|e| anyhow::anyhow!("failed to start service: {}", e))?;
    let port = read_serve_port().unwrap_or(DEFAULT_PORT);
    let host = if hosts_entry_present() {
        HOSTS_HOSTNAME
    } else {
        "localhost"
    };
    let url = format!("http://{}:{}", host, port);
    crate::output::fmt::serve_started(&url);
    Ok(())
}

/// Detiene el servicio (delega en el gestor de servicios).
pub fn cmd_serve_stop() -> Result<()> {
    if !is_installed() {
        anyhow::bail!("{}", t!("serve.error.not_installed"));
    }
    let manager = build_service_manager()?;
    manager
        .stop(ServiceStopCtx {
            label: service_label(),
        })
        .map_err(|e| anyhow::anyhow!("failed to stop service: {}", e))?;
    crate::output::fmt::serve_stopped();
    Ok(())
}

/// Muestra el estado del servicio: running/stopped y URL canónica.
pub fn cmd_serve_status() -> Result<()> {
    if !is_installed() {
        anyhow::bail!("{}", t!("serve.error.not_installed"));
    }

    let port = read_serve_port().unwrap_or(DEFAULT_PORT);
    let host = if hosts_entry_present() {
        HOSTS_HOSTNAME
    } else {
        "localhost"
    };
    let running = server_listening(port);
    let url = if running {
        Some(format!("http://{}:{}", host, port))
    } else {
        None
    };
    crate::output::fmt::serve_status(running, url.as_deref());
    Ok(())
}

/// Comando por defecto cuando se invoca `pillbox serve` sin subcomando.
///
/// Si el servicio está instalado, muestra el estado; si no, indica al
/// usuario que ejecute `pillbox serve install`.
pub fn cmd_serve_info() -> Result<()> {
    if is_installed() {
        cmd_serve_status()
    } else {
        println!("\n{}\n", t!("serve.error.not_installed").dimmed());
        Ok(())
    }
}

// ─── Helpers públicos ─────────────────────────────────────────────────────────

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

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn read_write_serve_port_roundtrip() {
        // Aislamos HOME para evitar tocar la config real del usuario.
        let tmp = tempfile::tempdir().unwrap();
        let home_lock = std::env::var("HOME").ok();
        std::env::set_var("HOME", tmp.path());

        // Reusa la API pública: write_serve_port + read_serve_port.
        write_serve_port(8080).unwrap();
        let got = read_serve_port();
        assert_eq!(got, Some(8080));

        if let Some(h) = home_lock {
            std::env::set_var("HOME", h);
        } else {
            std::env::remove_var("HOME");
        }
    }

    /// Reproduce la lógica de `write_hosts_entry` apuntando a un fichero
    /// temporal — evita tocar `/etc/hosts` en tests.
    fn write_hosts_entry_at(path: &std::path::Path) -> std::io::Result<()> {
        let current = std::fs::read_to_string(path).unwrap_or_default();
        if current.lines().any(|l| l.contains(HOSTS_MARKER)) {
            return Ok(());
        }
        let needs_newline = !current.is_empty() && !current.ends_with('\n');
        let mut new_content = current;
        if needs_newline {
            new_content.push('\n');
        }
        new_content.push_str(&format!(
            "127.0.0.1\t{}\t{}\n",
            HOSTS_HOSTNAME, HOSTS_MARKER
        ));
        std::fs::write(path, new_content)
    }

    /// Reproduce la lógica de `remove_hosts_entry` sobre un fichero arbitrario.
    fn remove_hosts_entry_at(path: &std::path::Path) -> std::io::Result<()> {
        let current = match std::fs::read_to_string(path) {
            Ok(s) => s,
            Err(_) => return Ok(()),
        };
        let new_content: String = current
            .lines()
            .filter(|l| !l.contains(HOSTS_MARKER))
            .map(|l| format!("{}\n", l))
            .collect();
        std::fs::write(path, new_content)
    }

    #[test]
    fn write_hosts_entry_is_idempotent() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("hosts");
        let mut f = std::fs::File::create(&path).unwrap();
        writeln!(f, "127.0.0.1\tlocalhost").unwrap();
        drop(f);

        write_hosts_entry_at(&path).unwrap();
        write_hosts_entry_at(&path).unwrap();

        let content = std::fs::read_to_string(&path).unwrap();
        let marker_count = content.lines().filter(|l| l.contains(HOSTS_MARKER)).count();
        assert_eq!(marker_count, 1, "marker should appear exactly once");
    }

    #[test]
    fn remove_hosts_entry_preserves_other_lines() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("hosts");
        let mut f = std::fs::File::create(&path).unwrap();
        writeln!(f, "127.0.0.1\tlocalhost").unwrap();
        writeln!(f, "::1\tlocalhost").unwrap();
        writeln!(f, "10.0.0.5\tmyserver").unwrap();
        drop(f);

        write_hosts_entry_at(&path).unwrap();
        remove_hosts_entry_at(&path).unwrap();

        let content = std::fs::read_to_string(&path).unwrap();
        assert!(!content.contains(HOSTS_MARKER), "marker line removed");
        assert!(content.contains("127.0.0.1\tlocalhost"));
        assert!(content.contains("::1\tlocalhost"));
        assert!(content.contains("10.0.0.5\tmyserver"));
    }
}
