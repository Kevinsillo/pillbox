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
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

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

/// Plazo máximo para que el puerto se libere o vuelva a escuchar al reiniciar.
const WAIT_TIMEOUT: Duration = Duration::from_secs(10);

/// Intervalo entre sondeos del puerto durante el reinicio.
const WAIT_INTERVAL: Duration = Duration::from_millis(100);

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
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    {
        service_definition_path()
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

/// Ruta del fichero de definición del servicio de usuario.
///
/// - Linux: `~/.config/systemd/user/pillbox-daemon.service`.
/// - macOS: `~/Library/LaunchAgents/pillbox-daemon.plist`.
#[cfg(any(target_os = "linux", target_os = "macos"))]
fn service_definition_path() -> Option<PathBuf> {
    let home = dirs::home_dir()?;
    #[cfg(target_os = "linux")]
    let path = home
        .join(".config")
        .join("systemd")
        .join("user")
        .join(format!("{}.service", SERVICE_LABEL));
    #[cfg(target_os = "macos")]
    let path = home
        .join("Library")
        .join("LaunchAgents")
        .join(format!("{}.plist", SERVICE_LABEL));
    Some(path)
}

/// Binario que ejecuta el servicio instalado, leído de su definición
/// (unit de systemd o plist de launchd). `None` si no está instalado, no se
/// puede leer o la plataforma no lo soporta (Windows).
#[cfg_attr(windows, allow(dead_code))]
pub(crate) fn installed_service_exe() -> Option<PathBuf> {
    #[cfg(target_os = "linux")]
    {
        let text = fs::read_to_string(service_definition_path()?).ok()?;
        parse_exec_start(&text)
    }

    #[cfg(target_os = "macos")]
    {
        let text = fs::read_to_string(service_definition_path()?).ok()?;
        parse_plist_program(&text)
    }

    #[cfg(not(any(target_os = "linux", target_os = "macos")))]
    {
        None
    }
}

/// Verifica que el proceso corre con privilegios de Administrador.
///
/// En Windows, instalar/desinstalar el servicio del SCM y escribir el fichero
/// `hosts` requieren elevación. Esta guarda se llama incondicionalmente al
/// inicio de `cmd_serve_install`/`cmd_serve_uninstall`; ambas variantes (`cfg`)
/// existen para que el call site compile limpio en todas las plataformas.
#[cfg(target_os = "windows")]
fn require_elevated() -> Result<()> {
    if !is_elevated::is_elevated() {
        anyhow::bail!("{}", t!("serve.error.requires_admin"));
    }
    Ok(())
}

/// No-op en plataformas distintas de Windows: systemd/launchd operan a nivel
/// de usuario y no requieren elevación.
#[cfg(not(target_os = "windows"))]
fn require_elevated() -> Result<()> {
    Ok(())
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
/// al fichero `hosts`. Solo se llama en Windows (proceso ya elevado para SCM).
/// En Linux/macOS se muestra un hint manual en su lugar.
#[cfg(target_os = "windows")]
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

/// Puerto efectivo del servicio: el persistido en `serve.port` o `DEFAULT_PORT`.
pub(crate) fn current_port() -> u16 {
    read_serve_port().unwrap_or(DEFAULT_PORT)
}

/// URL canónica del servicio: `pillbox.local` si la entrada de `hosts` existe,
/// `localhost` en caso contrario, con el puerto efectivo.
pub(crate) fn serve_url() -> String {
    let host = if hosts_entry_present() {
        HOSTS_HOSTNAME
    } else {
        "localhost"
    };
    format!("http://{}:{}", host, current_port())
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
    require_elevated()?;
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

    // On Windows the process is already elevated (required for SCM), so write
    // the hosts entry automatically. On Linux/macOS the service is a user-unit
    // and needs no privileges — show a copyable hint instead.
    #[cfg(target_os = "windows")]
    if let Err(e) = write_hosts_entry() {
        tracing::warn!("hosts write failed: {}", e);
        eprintln!("{} {}", "!".yellow().bold(), t!("serve.error.hosts_write"));
    }

    println!("\n{} {}\n", "✓".green().bold(), t!("serve.install.success"));

    #[cfg(not(target_os = "windows"))]
    println!(
        "  {}\n  sudo sh -c 'echo \"127.0.0.1\\tpillbox.local\\t# pillbox-serve\" >> /etc/hosts'\n",
        t!("serve.install.hosts_hint")
    );

    Ok(())
}

/// Desinstala el servicio del sistema.
///
/// Pasos:
/// 1. Elimina la entrada de `hosts` (no fatal).
/// 2. Llama a `ServiceManager::uninstall` (error si no estaba instalado).
/// 3. Borra `~/.pillbox/serve.port`.
pub fn cmd_serve_uninstall() -> Result<()> {
    require_elevated()?;
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

/// Arranca el servicio vía el gestor de servicios, sin imprimir nada.
fn service_start() -> Result<()> {
    let manager = build_service_manager()?;
    manager
        .start(ServiceStartCtx {
            label: service_label(),
        })
        .map_err(|e| anyhow::anyhow!("failed to start service: {}", e))
}

/// Detiene el servicio vía el gestor de servicios, sin imprimir nada.
fn service_stop() -> Result<()> {
    let manager = build_service_manager()?;
    manager
        .stop(ServiceStopCtx {
            label: service_label(),
        })
        .map_err(|e| anyhow::anyhow!("failed to stop service: {}", e))
}

/// Arranca el servicio (delega en el gestor de servicios).
pub fn cmd_serve_start() -> Result<()> {
    if !is_installed() {
        anyhow::bail!("{}", t!("serve.error.not_installed"));
    }
    service_start()?;
    crate::output::fmt::serve_started(&serve_url());
    Ok(())
}

/// Detiene el servicio (delega en el gestor de servicios).
pub fn cmd_serve_stop() -> Result<()> {
    if !is_installed() {
        anyhow::bail!("{}", t!("serve.error.not_installed"));
    }
    service_stop()?;
    crate::output::fmt::serve_stopped();
    Ok(())
}

/// Reinicia el servicio y confirma con la URL una vez vuelve a escuchar.
///
/// Si estaba parado, simplemente lo arranca (con la misma confirmación).
pub fn cmd_serve_restart() -> Result<()> {
    if !is_installed() {
        anyhow::bail!("{}", t!("serve.error.not_installed"));
    }
    restart_service()?;
    crate::output::fmt::serve_restarted(&serve_url());
    Ok(())
}

/// Núcleo silencioso del reinicio, reutilizado por `pillbox update`.
///
/// - En marcha: stop → espera a que el puerto quede libre (error
///   `stop_timeout` sin intentar arrancar) → start → espera a que escuche.
/// - Parado: start → espera a que escuche.
///
/// Error `start_timeout` si tras arrancar no escucha dentro del plazo.
pub(crate) fn restart_service() -> Result<()> {
    let port = current_port();
    if server_listening(port) {
        service_stop()?;
        if !wait_until(|| !server_listening(port), WAIT_TIMEOUT, WAIT_INTERVAL) {
            anyhow::bail!("{}", t!("serve.error.stop_timeout"));
        }
    }
    service_start()?;
    if !wait_until(|| server_listening(port), WAIT_TIMEOUT, WAIT_INTERVAL) {
        anyhow::bail!("{}", t!("serve.error.start_timeout"));
    }
    Ok(())
}

/// Muestra el estado del servicio: running/stopped y URL canónica.
pub fn cmd_serve_status() -> Result<()> {
    if !is_installed() {
        anyhow::bail!("{}", t!("serve.error.not_installed"));
    }

    let running = is_running();
    let url = if running { Some(serve_url()) } else { None };
    crate::output::fmt::serve_status(running, url.as_deref());
    Ok(())
}

/// Comando por defecto cuando se invoca `pillbox serve` sin subcomando.
///
/// Muestra la línea de estado del servicio (no instalado / parado / en marcha
/// con su URL) seguida de la ayuda con los subcomandos disponibles. Siempre
/// termina con éxito: es una consulta informativa.
pub fn cmd_serve_info() -> Result<()> {
    use crate::output::fmt::ServeState;
    let state = if !is_installed() {
        ServeState::NotInstalled
    } else if is_running() {
        ServeState::Running
    } else {
        ServeState::Stopped
    };
    crate::output::fmt::serve_status_with_help(
        state,
        &serve_url(),
        &crate::help::render_help("serve"),
    );
    Ok(())
}

// ─── Helpers públicos ─────────────────────────────────────────────────────────

/// Verifica si hay algo escuchando en `localhost:{port}` con un timeout de 200 ms.
pub fn server_listening(port: u16) -> bool {
    use std::net::TcpStream;
    TcpStream::connect_timeout(
        &std::net::SocketAddr::from(([127, 0, 0, 1], port)),
        Duration::from_millis(200),
    )
    .is_ok()
}

/// Indica si el servicio está escuchando en su puerto efectivo.
pub(crate) fn is_running() -> bool {
    server_listening(current_port())
}

/// Sondea `pred` cada `interval` hasta que devuelva `true` o venza `timeout`.
/// Devuelve `true` si la condición se cumplió dentro del plazo.
pub(crate) fn wait_until(
    mut pred: impl FnMut() -> bool,
    timeout: Duration,
    interval: Duration,
) -> bool {
    let deadline = Instant::now() + timeout;
    loop {
        if pred() {
            return true;
        }
        if Instant::now() >= deadline {
            return false;
        }
        std::thread::sleep(interval);
    }
}

/// Decisión de reinicio del servicio web tras una actualización.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RestartPlan {
    Restart,
    Skip,
}

/// Solo se reinicia si el servicio está instalado **y** escuchando.
pub(crate) fn restart_plan(installed: bool, listening: bool) -> RestartPlan {
    if installed && listening {
        RestartPlan::Restart
    } else {
        RestartPlan::Skip
    }
}

/// Extrae el ejecutable (primer token) de la línea `ExecStart=` de una unit
/// de systemd. Admite el token entre comillas dobles.
#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
pub(crate) fn parse_exec_start(unit: &str) -> Option<PathBuf> {
    let cmd = unit
        .lines()
        .find_map(|l| l.trim().strip_prefix("ExecStart="))?
        .trim();
    let token = match cmd.strip_prefix('"') {
        Some(rest) => rest.split('"').next()?,
        None => cmd.split_whitespace().next()?,
    };
    (!token.is_empty()).then(|| PathBuf::from(token))
}

/// Extrae el programa (primer `<string>` del array `ProgramArguments`) de un
/// plist de launchd. Parseo por subcadenas: suficiente para el plist que
/// genera `service-manager`.
#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
pub(crate) fn parse_plist_program(plist: &str) -> Option<PathBuf> {
    let after_key = &plist[plist.find("<key>ProgramArguments</key>")?..];
    let after_array = &after_key[after_key.find("<array>")?..];
    let start = after_array.find("<string>")? + "<string>".len();
    let end = start + after_array[start..].find("</string>")?;
    let raw = after_array[start..end].trim();
    if raw.is_empty() {
        return None;
    }
    let value = raw
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&apos;", "'")
        .replace("&amp;", "&");
    Some(PathBuf::from(value))
}

/// Compara el binario del servicio instalado con el actual. `None` si no se
/// conoce el del servicio; canonicaliza ambas rutas cuando es posible.
#[cfg_attr(windows, allow(dead_code))]
pub(crate) fn exe_matches(installed: Option<&Path>, current: &Path) -> Option<bool> {
    let installed = installed?;
    let canon = |p: &Path| fs::canonicalize(p).unwrap_or_else(|_| p.to_path_buf());
    Some(canon(installed) == canon(current))
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

    #[test]
    fn wait_until_returns_true_immediately_when_pred_holds() {
        let mut calls = 0;
        let ok = wait_until(
            || {
                calls += 1;
                true
            },
            Duration::from_millis(50),
            Duration::from_millis(1),
        );
        assert!(ok);
        assert_eq!(calls, 1);
    }

    #[test]
    fn wait_until_returns_true_when_pred_becomes_true() {
        let mut calls = 0;
        let ok = wait_until(
            || {
                calls += 1;
                calls >= 3
            },
            Duration::from_secs(1),
            Duration::from_millis(1),
        );
        assert!(ok);
        assert_eq!(calls, 3);
    }

    #[test]
    fn wait_until_returns_false_on_timeout() {
        let start = Instant::now();
        let ok = wait_until(
            || false,
            Duration::from_millis(30),
            Duration::from_millis(5),
        );
        assert!(!ok);
        assert!(start.elapsed() >= Duration::from_millis(30));
    }

    #[test]
    fn restart_plan_only_restarts_when_installed_and_listening() {
        assert_eq!(restart_plan(true, true), RestartPlan::Restart);
        assert_eq!(restart_plan(true, false), RestartPlan::Skip);
        assert_eq!(restart_plan(false, true), RestartPlan::Skip);
        assert_eq!(restart_plan(false, false), RestartPlan::Skip);
    }

    #[test]
    fn parse_exec_start_takes_first_token() {
        let unit = "[Unit]\nDescription=pillbox-daemon\n\n[Service]\nExecStart=/home/u/.local/bin/pillbox serve run --port 4242\nRestart=on-failure\n";
        assert_eq!(
            parse_exec_start(unit),
            Some(PathBuf::from("/home/u/.local/bin/pillbox"))
        );
    }

    #[test]
    fn parse_exec_start_handles_quoted_path() {
        let unit = "[Service]\nExecStart=\"/opt/my apps/pillbox\" serve run\n";
        assert_eq!(
            parse_exec_start(unit),
            Some(PathBuf::from("/opt/my apps/pillbox"))
        );
    }

    #[test]
    fn parse_exec_start_missing_returns_none() {
        assert_eq!(parse_exec_start("[Service]\nRestart=always\n"), None);
        assert_eq!(parse_exec_start("[Service]\nExecStart=\n"), None);
    }

    #[test]
    fn parse_plist_program_takes_first_string_of_program_arguments() {
        let plist = r#"<?xml version="1.0" encoding="UTF-8"?>
<plist version="1.0">
<dict>
	<key>Label</key>
	<string>pillbox-daemon</string>
	<key>ProgramArguments</key>
	<array>
		<string>/Users/u/.local/bin/pill&amp;box</string>
		<string>serve</string>
		<string>run</string>
	</array>
</dict>
</plist>"#;
        assert_eq!(
            parse_plist_program(plist),
            Some(PathBuf::from("/Users/u/.local/bin/pill&box"))
        );
    }

    #[test]
    fn parse_plist_program_missing_returns_none() {
        let plist = "<dict><key>Label</key><string>pillbox-daemon</string></dict>";
        assert_eq!(parse_plist_program(plist), None);
    }

    #[test]
    fn exe_matches_compares_paths() {
        let dir = tempfile::tempdir().unwrap();
        let a = dir.path().join("a");
        let b = dir.path().join("b");
        std::fs::write(&a, "").unwrap();
        std::fs::write(&b, "").unwrap();
        let a_indirect = dir.path().join(".").join("a");

        assert_eq!(exe_matches(None, &a), None);
        assert_eq!(exe_matches(Some(&a), &a), Some(true));
        assert_eq!(exe_matches(Some(&a_indirect), &a), Some(true));
        assert_eq!(exe_matches(Some(&b), &a), Some(false));
        assert_eq!(
            exe_matches(
                Some(Path::new("/nonexistent/x")),
                Path::new("/nonexistent/y")
            ),
            Some(false)
        );
    }
}
