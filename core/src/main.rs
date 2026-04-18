mod exec;
mod i18n;
mod output;
mod server;

use anyhow::{Context, Result};
use clap::{CommandFactory, Parser, Subcommand};
use indicatif::{ProgressBar, ProgressStyle};
use owo_colors::OwoColorize;
use rust_i18n::t;
use std::time::Duration;

rust_i18n::i18n!("locales", fallback = "es");

// ─── CLI ──────────────────────────────────────────────────────────────────────

#[derive(Parser)]
#[command(
    name = "pillbox",
    version,
    about = "Persistent knowledge memory for AI agents",
    disable_help_subcommand = true
)]
struct Cli {
    /// Inicializa la DB global (~/.pillbox/pillbox.db). Llamado por install.sh.
    #[arg(long, hide = true)]
    init_global: bool,

    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand)]
enum Command {
    /// Estado global: DBs, bottle activo, servidor, MCP y skill.
    Status,

    /// Lee JSON de stdin, ejecuta la operación y escribe JSON en stdout (usado por el MCP).
    #[command(hide = true)]
    Exec,

    /// Gestiona el servidor HTTP.
    Serve {
        #[command(subcommand)]
        cmd: Option<ServeCommand>,
    },

    /// Operaciones sobre bottles.
    Bottle {
        #[command(subcommand)]
        cmd: Option<BottleCommand>,
    },

    /// Operaciones sobre pills del bottle actual.
    Pills {
        #[command(subcommand)]
        cmd: Option<PillsCommand>,
    },

    /// Operaciones sobre prescriptions del bottle actual.
    Prescription {
        #[command(subcommand)]
        cmd: Option<PrescriptionCommand>,
    },

    /// Gestiona el servidor MCP.
    Mcp {
        #[command(subcommand)]
        cmd: Option<McpCommand>,
    },

    /// Gestiona la skill de Claude Code.
    Skill {
        #[command(subcommand)]
        cmd: Option<SkillCommand>,
    },

    /// Desinstala componentes de Pillbox.
    Uninstall,

    /// Cambia o muestra el idioma del CLI.
    Lang {
        #[command(subcommand)]
        cmd: Option<LangCommand>,
    },

    /// Muestra esta ayuda.
    #[command(hide = true)]
    Help,
}

#[derive(Subcommand)]
enum ServeCommand {
    /// Arranca el servidor HTTP.
    Start {
        #[arg(short, long, default_value = "4242")]
        port: u16,
        /// Ejecuta en segundo plano (daemon).
        #[arg(short, long)]
        daemon: bool,
    },
    /// Para el servidor daemon.
    Stop,
    /// Muestra el estado del servidor.
    Status,
}

#[derive(Subcommand)]
enum BottleCommand {
    /// Inicializa un bottle en el directorio actual (wizard interactivo).
    Init,

    /// Estado del bottle del directorio actual.
    Status,

    /// Lista los bottles registrados en la DB global.
    List,

    /// Migra el bottle entre DB local y global.
    Migrate {
        #[command(subcommand)]
        subcommand: Option<MigrateCommand>,
    },
}

#[derive(Subcommand)]
enum MigrateCommand {
    /// Mueve el bottle de este directorio a la DB global.
    Global,
    /// Elige un bottle de la DB global y muévelo aquí.
    Local,
}

#[derive(Subcommand)]
enum PrescriptionCommand {
    /// Abre una nueva prescripción para el bottle actual.
    Open {
        /// Título de la tarea o funcionalidad.
        title: String,
    },

    /// Lista las prescriptions del bottle actual (más recientes primero).
    List {
        #[arg(short, long, default_value = "10")]
        limit: u32,
    },

    /// Cierra la prescripción abierta del bottle actual.
    Close,
}

#[derive(Subcommand)]
enum PillsCommand {
    /// Lista todas las pills del bottle actual.
    List,
}

#[derive(Subcommand)]
enum McpCommand {
    /// Instala el servidor MCP en ~/.pillbox/mcp/.
    Install,
    /// Desinstala el servidor MCP.
    Uninstall,
}

#[derive(Subcommand)]
enum SkillCommand {
    /// Instala la skill de Claude Code en ~/.claude/skills/pillbox/.
    Install,
    /// Desinstala la skill de Claude Code.
    Uninstall,
}

#[derive(Subcommand)]
enum LangCommand {
    /// Cambia el idioma del CLI (es, en, de, it, pt, fr).
    Set {
        /// Código de idioma.
        code: String,
    },
}

// ─── Entry point ─────────────────────────────────────────────────────────────

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(tracing::Level::WARN.into()),
        )
        .init();

    rust_i18n::set_locale(&i18n::detect());

    let cli = Cli::parse();

    if cli.init_global {
        return cmd_init_global();
    }

    match cli.command {
        None => cmd_root_help(),
        Some(Command::Status) => cmd_status(),
        Some(Command::Exec) => exec::run(),
        Some(Command::Serve { cmd }) => match cmd {
            Some(ServeCommand::Start { port, daemon }) => cmd_serve_start(port, daemon).await,
            Some(ServeCommand::Stop) => cmd_serve_stop(),
            Some(ServeCommand::Status) => cmd_serve_status(),
            None => cmd_serve_info(),
        },
        Some(Command::Bottle { cmd }) => match cmd {
            Some(BottleCommand::Init) => cmd_bottle_init(),
            Some(BottleCommand::Status) => cmd_bottle_status(),
            Some(BottleCommand::List) => cmd_bottle_list(),
            Some(BottleCommand::Migrate { subcommand }) => match subcommand {
                None => cmd_migrate_help(),
                Some(MigrateCommand::Global) => cmd_migrate_global(),
                Some(MigrateCommand::Local) => cmd_migrate_local(),
            },
            None => cmd_sub_help("bottle"),
        },
        Some(Command::Pills { cmd }) => match cmd {
            Some(PillsCommand::List) => cmd_pills_list(),
            None => cmd_sub_help("pills"),
        },
        Some(Command::Prescription { cmd }) => match cmd {
            Some(PrescriptionCommand::Open { title }) => cmd_prescription_open(title),
            Some(PrescriptionCommand::List { limit }) => cmd_prescription_list(limit),
            Some(PrescriptionCommand::Close) => cmd_prescription_close(),
            None => cmd_sub_help("prescription"),
        },
        Some(Command::Mcp { cmd }) => match cmd {
            Some(McpCommand::Install) => cmd_mcp_install(),
            Some(McpCommand::Uninstall) => cmd_mcp_uninstall(),
            None => cmd_mcp_status(),
        },
        Some(Command::Skill { cmd }) => match cmd {
            Some(SkillCommand::Install) => cmd_skill_install(),
            Some(SkillCommand::Uninstall) => cmd_skill_uninstall(),
            None => cmd_skill_status(),
        },
        Some(Command::Uninstall) => cmd_uninstall(),
        Some(Command::Lang { cmd }) => match cmd {
            Some(LangCommand::Set { code }) => cmd_lang_set(code),
            None => cmd_lang_show(),
        },
        Some(Command::Help) => cmd_root_help(),
    }
}

// ─── cmd_init_global ─────────────────────────────────────────────────────────

fn cmd_init_global() -> Result<()> {
    let path = pillbox::config::global_db_path();
    pillbox::db::connection::open(&path)?;
    Ok(())
}

// ─── cmd_bottle_list ─────────────────────────────────────────────────────────

fn cmd_bottle_list() -> Result<()> {
    use pillbox::db::{connection, store::bottles};

    let path = pillbox::config::global_db_path();
    if !path.exists() {
        output::fmt::db_not_found();
        return Ok(());
    }

    let conn = connection::open(&path)?;
    let all = bottles::list(&conn)?;
    output::fmt::bottles_list(&all);
    Ok(())
}

// ─── cmd_status ──────────────────────────────────────────────────────────────

fn cmd_status() -> Result<()> {
    use output::fmt::{StatusBottle, StatusDb};

    let bin_path = std::env::current_exe()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|_| t!("status.bin_unknown").to_string());

    let query_db = |path: &std::path::Path| -> Option<Result<(i64, i64, i64, i64, i64), String>> {
        if !path.exists() {
            return None;
        }
        Some((|| -> Result<_, String> {
            let conn = pillbox::db::connection::open(path).map_err(|e| e.to_string())?;
            conn.query_row(
                "SELECT
                         (SELECT MAX(version) FROM schema_migrations),
                         (SELECT COUNT(*) FROM bottles),
                         (SELECT COUNT(*) FROM pills         WHERE deleted_at IS NULL),
                         (SELECT COUNT(*) FROM capsules      WHERE deleted_at IS NULL),
                         (SELECT COUNT(*) FROM prescriptions WHERE deleted_at IS NULL)",
                [],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?)),
            )
            .map_err(|e| e.to_string())
        })())
    };

    let global_path = pillbox::config::global_db_path();
    let local_path = pillbox::config::local_db_path();

    let global = StatusDb {
        result: query_db(&global_path),
        path: global_path.display().to_string(),
    };
    let local = StatusDb {
        result: query_db(&local_path),
        path: local_path.display().to_string(),
    };

    let bottle = pillbox::config::resolve_db_path()
        .and_then(|db_path| pillbox::db::connection::open(&db_path).ok())
        .and_then(|conn| {
            let dir = std::env::current_dir().ok()?.to_string_lossy().to_string();
            pillbox::db::store::bottles::find_by_directory(&conn, &dir)
                .ok()
                .flatten()
                .map(|b| {
                    let open_rx = conn
                        .query_row(
                            "SELECT title FROM prescriptions
                             WHERE bottle_id = ?1 AND ended_at IS NULL AND deleted_at IS NULL
                             LIMIT 1",
                            rusqlite::params![b.id],
                            |r| r.get(0),
                        )
                        .ok();
                    StatusBottle { open_rx }
                })
        });

    let server_port = {
        use std::net::TcpStream;
        use std::time::Duration;
        let port = pillbox::config::DEFAULT_PORT;
        TcpStream::connect_timeout(
            &std::net::SocketAddr::from(([127, 0, 0, 1], port)),
            Duration::from_millis(150),
        )
        .ok()
        .map(|_| port)
    };

    output::fmt::status(
        &bin_path,
        global,
        local,
        bottle,
        server_port,
        &pillbox::config::mcp_path(),
        &pillbox::config::skill_path(),
    );
    Ok(())
}

// ─── cmd_bottle_init ─────────────────────────────────────────────────────────

fn cmd_bottle_init() -> Result<()> {
    use inquire::{Confirm, Select, Text};
    use pillbox::db::{connection, store::bottles};
    use pillbox::domain::bottle::{BottleScope, NewBottle};

    let current_dir = std::env::current_dir()?;
    let dir_str = current_dir.to_string_lossy().to_string();
    let dir_basename = current_dir
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();

    output::fmt::bottle_init_start(&dir_str);

    let display_name = Text::new(&t!("bottle.init.name_prompt"))
        .with_default(&dir_basename)
        .prompt()?;

    let name = pillbox::normalize::bottle_name(&display_name);

    let scope_options = vec![
        t!("bottle.init.scope.local").to_string(),
        t!("bottle.init.scope.global").to_string(),
    ];
    let scope_choice =
        Select::new(&t!("bottle.init.scope.prompt"), scope_options).prompt()?;
    let scope = if scope_choice.starts_with("local") {
        BottleScope::Local
    } else {
        BottleScope::Global
    };

    let db_path = match &scope {
        BottleScope::Local => pillbox::config::local_db_path(),
        BottleScope::Global => pillbox::config::global_db_path(),
    };

    let pb = spinner(t!("bottle.init.creating"));
    let mut conn = connection::open(&db_path)?;

    if bottles::find_by_directory(&conn, &dir_str)?.is_some() {
        pb.finish_and_clear();
        anyhow::bail!("{}", t!("bottle.init.exists", path = db_path.display()));
    }

    let bottle = bottles::create(
        &mut conn,
        &NewBottle {
            name: name.clone(),
            display_name: display_name.clone(),
            directory: dir_str.clone(),
            scope: scope.clone(),
        },
    )?;
    pb.finish_and_clear();

    output::fmt::bottle_init_created(&bottle.name, &bottle.display_name, &db_path);

    if scope == BottleScope::Local {
        if current_dir.join(".git").exists() {
            let add_gi = Confirm::new(&t!("bottle.init.gitignore.prompt"))
                .with_default(false)
                .prompt()
                .unwrap_or(false);
            if add_gi {
                add_to_gitignore(&current_dir)?;
                output::fmt::bottle_init_gitignore();
            }
        }

        let global_path = pillbox::config::global_db_path();
        if global_path.exists() {
            let pb2 = spinner(t!("bottle.init.registering"));
            match register_in_global(&global_path, &name, &display_name, &dir_str, &scope) {
                Ok(_) => pb2.finish_with_message(t!("bottle.init.registered_ok").to_string()),
                Err(e) => {
                    pb2.finish_and_clear();
                    eprintln!("{}", t!("bottle.init.register_err", err = e));
                }
            }
        }
    }

    output::fmt::bottle_init_done();
    Ok(())
}

fn add_to_gitignore(dir: &std::path::Path) -> Result<()> {
    use std::io::Write;
    let gi_path = dir.join(".gitignore");
    let entry = ".pillbox/\n";

    if gi_path.exists() {
        let content = std::fs::read_to_string(&gi_path)?;
        if content
            .lines()
            .any(|l| l.trim() == ".pillbox/" || l.trim() == ".pillbox")
        {
            return Ok(());
        }
        let mut file = std::fs::OpenOptions::new().append(true).open(&gi_path)?;
        if !content.ends_with('\n') {
            file.write_all(b"\n")?;
        }
        file.write_all(entry.as_bytes())?;
    } else {
        std::fs::write(&gi_path, entry)?;
    }
    Ok(())
}

fn register_in_global(
    global_path: &std::path::Path,
    name: &str,
    display_name: &str,
    directory: &str,
    scope: &pillbox::domain::bottle::BottleScope,
) -> Result<()> {
    use pillbox::db::{connection, store::bottles};
    use pillbox::domain::bottle::NewBottle;

    let mut conn = connection::open(global_path)?;
    if bottles::find_by_directory(&conn, directory)?.is_none() {
        bottles::create(
            &mut conn,
            &NewBottle {
                name: name.to_string(),
                display_name: display_name.to_string(),
                directory: directory.to_string(),
                scope: scope.clone(),
            },
        )?;
    }
    Ok(())
}

// ─── cmd_bottle_status ───────────────────────────────────────────────────────

fn cmd_bottle_status() -> Result<()> {
    let (conn, _) = open_resolved_db()?;
    let bottle = find_current_bottle()?;

    let pill_count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM pills p
         JOIN prescriptions rx ON rx.id = p.prescription_id
         WHERE rx.bottle_id = ?1 AND p.deleted_at IS NULL",
        rusqlite::params![bottle.id],
        |r| r.get(0),
    )?;

    let open_rx: Option<(String, String)> = conn
        .query_row(
            "SELECT id, title FROM prescriptions
             WHERE bottle_id = ?1 AND ended_at IS NULL AND deleted_at IS NULL
             LIMIT 1",
            rusqlite::params![bottle.id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .ok();

    output::fmt::bottle_status(&bottle, pill_count, open_rx);
    Ok(())
}

// ─── cmd_pills_list ──────────────────────────────────────────────────────────

fn cmd_pills_list() -> Result<()> {
    use pillbox::db::{connection, store::bottles};

    let global_path = pillbox::config::global_db_path();
    if !global_path.exists() {
        output::fmt::db_not_found();
        return Ok(());
    }

    let global_conn = connection::open(&global_path)?;
    let bottle = match find_current_bottle() {
        Ok(b) => b,
        Err(_) => {
            println!("{}\n", t!("bottle.error.not_found_short"));
            let all = bottles::list(&global_conn)?;
            output::fmt::bottles_list(&all);
            return Ok(());
        }
    };

    let conn = pillbox::db::connection::open(
        &pillbox::config::resolve_db_path().unwrap_or_else(|| global_path.clone()),
    )?;

    let mut stmt = conn.prepare(
        "SELECT p.id, p.compound, p.title, p.created_at
         FROM pills p
         JOIN prescriptions rx ON rx.id = p.prescription_id
         WHERE rx.bottle_id = ?1 AND p.deleted_at IS NULL
         ORDER BY p.created_at DESC",
    )?;

    let pills: Vec<(i64, String, String, String)> = stmt
        .query_map(rusqlite::params![bottle.id], |r| {
            Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?))
        })?
        .collect::<rusqlite::Result<_>>()?;

    output::fmt::pills_list(&bottle.name, &pills);
    Ok(())
}

// ─── cmd_prescription_open ───────────────────────────────────────────────────

fn cmd_prescription_open(title: String) -> Result<()> {
    use pillbox::db::store::{prescriptions, PrescriptionAlreadyOpen};
    use pillbox::domain::prescription::NewPrescription;

    let (mut conn, _) = open_resolved_db()?;
    let bottle = find_current_bottle()?;

    match prescriptions::open(
        &mut conn,
        &NewPrescription {
            bottle_id: bottle.id,
            title,
        },
    ) {
        Ok(rx) => output::fmt::prescription_opened(&rx.id, &rx.title),
        Err(e) => {
            if let Some(already) = e.downcast_ref::<PrescriptionAlreadyOpen>() {
                anyhow::bail!(
                    "{}",
                    t!(
                        "prescriptions.error.already_open",
                        title = already.title,
                        pills = already.pill_count,
                        id = &already.id[..already.id.len().min(8)]
                    )
                );
            }
            return Err(e);
        }
    }
    Ok(())
}

// ─── cmd_prescription_list ───────────────────────────────────────────────────

fn cmd_prescription_list(limit: u32) -> Result<()> {
    use pillbox::db::store::prescriptions;

    let (conn, _) = open_resolved_db()?;
    let bottle = find_current_bottle()?;
    let rxs = prescriptions::list_by_bottle(&conn, bottle.id, limit)?;

    output::fmt::prescriptions_list(&bottle.name, &rxs, limit);
    Ok(())
}

// ─── cmd_prescription_close ──────────────────────────────────────────────────

fn cmd_prescription_close() -> Result<()> {
    use pillbox::db::store::prescriptions;

    let (mut conn, _) = open_resolved_db()?;
    let bottle = find_current_bottle()?;

    let open_rx: Option<(String, String)> = conn
        .query_row(
            "SELECT id, title FROM prescriptions
             WHERE bottle_id = ?1 AND ended_at IS NULL AND deleted_at IS NULL
             LIMIT 1",
            rusqlite::params![bottle.id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .ok();

    let (rx_id, rx_title) = open_rx.ok_or_else(|| {
        anyhow::anyhow!("{}", t!("prescriptions.error.none_open", name = bottle.name))
    })?;

    prescriptions::close(&mut conn, &rx_id)?;
    output::fmt::prescription_closed(&rx_title);
    Ok(())
}

// ─── cmd_serve_start ─────────────────────────────────────────────────────────

async fn cmd_serve_start(port: u16, daemon: bool) -> Result<()> {
    let pid_path = pillbox::config::pid_path();

    if let Some(pid) = read_pid(&pid_path) {
        if process_alive(pid) {
            anyhow::bail!("{}", t!("serve.error.already_running", pid = pid));
        }
        let _ = std::fs::remove_file(&pid_path);
    }

    let db_path = pillbox::config::resolve_db_path()
        .ok_or_else(|| anyhow::anyhow!("{}", t!("serve.error.no_db")))?;

    if daemon {
        let exe = std::env::current_exe()?;
        let child = std::process::Command::new(exe)
            .args(["serve", "start", "--port", &port.to_string()])
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()?;
        std::fs::write(&pid_path, child.id().to_string())?;
        println!(
            "{} {}\n",
            "●".green().bold(),
            t!("serve.daemon_started", pid = child.id(), port = port)
        );
    } else {
        std::fs::write(&pid_path, std::process::id().to_string())?;
        let result = server::run(port, db_path).await;
        let _ = std::fs::remove_file(&pid_path);
        result?;
    }
    Ok(())
}

// ─── cmd_serve_stop ──────────────────────────────────────────────────────────

fn cmd_serve_stop() -> Result<()> {
    let pid_path = pillbox::config::pid_path();
    let pid = read_pid(&pid_path)
        .ok_or_else(|| anyhow::anyhow!("{}", t!("serve.stop.none")))?;

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
    println!("{} {}\n", "●".green().bold(), t!("serve.stop.done", pid = pid));
    Ok(())
}

// ─── cmd_serve_status ────────────────────────────────────────────────────────

fn cmd_serve_status() -> Result<()> {
    let pid_path = pillbox::config::pid_path();
    let pid = read_pid(&pid_path);
    let running = pid.map(process_alive).unwrap_or(false);
    let port = pillbox::config::DEFAULT_PORT;
    output::fmt::serve_status(running, pid, port);
    Ok(())
}

// ─── serve helpers ───────────────────────────────────────────────────────────

fn read_pid(path: &std::path::Path) -> Option<u32> {
    std::fs::read_to_string(path).ok()?.trim().parse().ok()
}

fn process_alive(pid: u32) -> bool {
    #[cfg(unix)]
    {
        std::process::Command::new("kill")
            .args(["-0", &pid.to_string()])
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    }
    #[cfg(not(unix))]
    {
        false
    }
}

// ─── cmd_migrate_help ────────────────────────────────────────────────────────

fn cmd_migrate_help() -> Result<()> {
    use pillbox::db::{connection, store::bottles};

    let global_path = pillbox::config::global_db_path();
    let local_path = pillbox::config::local_db_path();

    // Intentar detectar bottle local del directorio actual
    let bottle_name: Option<String> = if global_path.exists() {
        let conn = connection::open(&global_path).ok();
        conn.and_then(|c| {
            let dir = std::env::current_dir().ok()?.to_string_lossy().to_string();
            bottles::find_by_directory(&c, &dir).ok().flatten().map(|b| b.name)
        })
    } else {
        None
    };

    output::fmt::migrate_help(
        bottle_name.as_deref(),
        &local_path.display().to_string(),
        &global_path.display().to_string(),
    );
    Ok(())
}

// ─── cmd_migrate_global ──────────────────────────────────────────────────────

fn cmd_migrate_global() -> Result<()> {
    use inquire::Confirm;
    use pillbox::db::{connection, migrate};

    let global_path = pillbox::config::global_db_path();
    let local_path = pillbox::config::local_db_path();

    if !global_path.exists() {
        anyhow::bail!("{}", t!("migrate.error.no_global", path = global_path.display()));
    }
    if !local_path.exists() {
        anyhow::bail!("{}", t!("migrate.error.no_local"));
    }

    // Leer bottle del directorio actual desde DB local
    let src_conn = connection::open(&local_path)?;
    let current_dir = std::env::current_dir()?;
    let dir_str = current_dir.to_string_lossy();

    let bottle_name: String = src_conn
        .query_row(
            "SELECT name FROM bottles WHERE directory = ?1",
            rusqlite::params![dir_str.as_ref()],
            |r| r.get(0),
        )
        .map_err(|_| {
            anyhow::anyhow!(
                "{}",
                t!("migrate.error.no_bottle", dir = dir_str, path = local_path.display())
            )
        })?;

    let (prescriptions, pills) = migrate::count_bottle_contents(&src_conn, &bottle_name)?;

    output::fmt::migrate_confirm_global(
        &bottle_name,
        &local_path.display().to_string(),
        &global_path.display().to_string(),
        prescriptions,
        pills,
    );

    let confirmed = Confirm::new(&t!("migrate.confirm.prompt"))
        .with_default(false)
        .prompt()?;

    if !confirmed {
        return Ok(());
    }

    let mut dst_conn = connection::open(&global_path)?;
    let result = migrate::migrate_bottle(&src_conn, &mut dst_conn, &bottle_name)?;
    drop(src_conn);

    std::fs::remove_file(&local_path)
        .with_context(|| "no se pudo eliminar la DB local tras la migración")?;

    output::fmt::migrate_result_global(result.prescriptions, result.pills);
    Ok(())
}

// ─── cmd_migrate_local ───────────────────────────────────────────────────────

fn cmd_migrate_local() -> Result<()> {
    use inquire::{Confirm, Select};
    use pillbox::db::{connection, migrate, store::bottles};

    let global_path = pillbox::config::global_db_path();
    let local_path = pillbox::config::local_db_path();

    if !global_path.exists() {
        anyhow::bail!("{}", t!("migrate.error.no_global", path = global_path.display()));
    }

    // Si existe DB local con un bottle → error
    if local_path.exists() {
        let local_conn = connection::open(&local_path).ok();
        let has_bottle = local_conn.and_then(|c| {
            let dir = std::env::current_dir().ok()?.to_string_lossy().to_string();
            bottles::find_by_directory(&c, &dir).ok().flatten()
        });
        if has_bottle.is_some() {
            anyhow::bail!("{}", t!("migrate.local.error.already_local"));
        }
    }

    let global_conn = connection::open(&global_path)?;
    let bottle_list = migrate::list_bottles_with_counts(&global_conn)?;

    if bottle_list.is_empty() {
        anyhow::bail!("{}", t!("migrate.local.error.no_bottles"));
    }

    // Construir opciones de Select
    let options: Vec<String> = bottle_list
        .iter()
        .map(|(name, dir, pill_count)| {
            format!("{}  —  {}  ({} pills)", name, dir, pill_count)
        })
        .collect();

    let selection = Select::new(&t!("migrate.local.select"), options.clone()).prompt()?;

    // Encontrar el bottle seleccionado por índice
    let idx = options.iter().position(|o| o == &selection).unwrap_or(0);
    let (bottle_name, _bottle_dir, _) = &bottle_list[idx];
    let bottle_name = bottle_name.clone();

    let (prescriptions, pills) = migrate::count_bottle_contents(&global_conn, &bottle_name)?;

    let will_create = !local_path.exists();

    output::fmt::migrate_confirm_local(
        &bottle_name,
        &local_path.display().to_string(),
        &global_path.display().to_string(),
        prescriptions,
        pills,
        will_create,
    );

    let confirmed = Confirm::new(&t!("migrate.confirm.prompt"))
        .with_default(false)
        .prompt()?;

    if !confirmed {
        return Ok(());
    }

    let mut dst_conn = connection::open(&local_path)?;
    let result = migrate::migrate_bottle(&global_conn, &mut dst_conn, &bottle_name)?;
    drop(global_conn);

    let mut global_conn_mut = connection::open(&global_path)?;
    migrate::delete_bottle(&mut global_conn_mut, &bottle_name)?;

    output::fmt::migrate_result_local(result.prescriptions, result.pills);
    Ok(())
}

// ─── cmd_mcp_status / cmd_skill_status ───────────────────────────────────────

fn render_help_cmd(cmd: &mut clap::Command, name: &str) -> String {
    let mut out = String::new();
    if let Some(about) = cmd.get_about() {
        out.push_str(&format!("{}\n\n", about.to_string().bold()));
    }
    out.push_str(&format!(
        "{} {} {}\n",
        t!("help.usage").bold(),
        name,
        "[COMMAND]".dimmed()
    ));
    let subcmds: Vec<_> = cmd.get_subcommands().filter(|s| !s.is_hide_set()).collect();
    if !subcmds.is_empty() {
        out.push_str(&format!("\n{}:\n", t!("help.commands").bold()));
        let max = subcmds
            .iter()
            .map(|s| s.get_name().len())
            .max()
            .unwrap_or(0);
        for s in &subcmds {
            let about = s.get_about().map(|a| a.to_string()).unwrap_or_default();
            out.push_str(&format!(
                "  {:<width$}  {}\n",
                s.get_name().green(),
                about,
                width = max
            ));
        }
    }
    out
}

fn render_help(subcmd: &str) -> String {
    let mut cmd = Cli::command();
    let sub = cmd.find_subcommand_mut(subcmd).unwrap();
    render_help_cmd(sub, subcmd)
}

fn render_root_help() -> String {
    let mut cmd = Cli::command();
    render_help_cmd(&mut cmd, "pillbox")
}

fn cmd_root_help() -> Result<()> {
    let rows = vec![["".to_string(), render_root_help()]];
    println!("{}\n", output::table::dict(rows));
    Ok(())
}

fn cmd_sub_help(subcmd: &str) -> Result<()> {
    let rows = vec![["".to_string(), render_help(subcmd)]];
    println!("{}\n", output::table::dict(rows));
    Ok(())
}

fn cmd_mcp_status() -> Result<()> {
    output::fmt::component_status_with_help(&pillbox::config::mcp_path(), &render_help("mcp"));
    Ok(())
}

fn cmd_skill_status() -> Result<()> {
    output::fmt::component_status_with_help(&pillbox::config::skill_path(), &render_help("skill"));
    Ok(())
}

fn cmd_serve_info() -> Result<()> {
    let pid_path = pillbox::config::pid_path();
    let pid = read_pid(&pid_path);
    let running = pid.map(process_alive).unwrap_or(false);
    let port = pillbox::config::DEFAULT_PORT;
    let status = if running {
        format!(
            "{} {}",
            "●".green(),
            t!("serve.inline.running", port = port, pid = pid.unwrap())
        )
    } else {
        format!("{} {}", "●".red(), t!("serve.inline.stopped", port = port))
    };
    let rows = vec![
        [t!("serve.title").bold().to_string(), status],
        ["".to_string(), render_help("serve")],
    ];
    println!("{}\n", output::table::dict(rows));
    Ok(())
}

// ─── cmd_mcp_install ─────────────────────────────────────────────────────────

fn cmd_mcp_install() -> Result<()> {
    let version = env!("CARGO_PKG_VERSION");
    let mcp_dir = pillbox::config::mcp_path().parent().unwrap().to_path_buf();
    let url = format!(
        "https://github.com/kevinsillo/pillbox/releases/download/v{}/pillbox-mcp.tar.gz",
        version
    );

    let node_ok = std::process::Command::new("node")
        .arg("--version")
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .and_then(|v| {
            v.trim()
                .trim_start_matches('v')
                .split('.')
                .next()?
                .parse::<u32>()
                .ok()
        })
        .map(|maj| maj >= 18)
        .unwrap_or(false);

    if !node_ok {
        anyhow::bail!("{}", t!("mcp.node_required"));
    }

    std::fs::create_dir_all(&mcp_dir)?;
    let pb = spinner(t!("mcp.downloading"));
    let status = std::process::Command::new("curl")
        .args(["-fsSL", &url, "--output", "/tmp/pillbox-mcp.tar.gz"])
        .status()?;
    if !status.success() {
        pb.finish_and_clear();
        anyhow::bail!("{}", t!("mcp.error.download", url = url));
    }
    let status = std::process::Command::new("tar")
        .args([
            "-xzf",
            "/tmp/pillbox-mcp.tar.gz",
            "-C",
            mcp_dir.to_str().unwrap(),
            "--strip-components=1",
        ])
        .status()?;
    pb.finish_and_clear();
    if !status.success() {
        anyhow::bail!("{}", t!("mcp.error.extract"));
    }
    println!("{} {}\n", "●".green().bold(), t!("mcp.installed", path = mcp_dir.display()));
    println!("{}\n", t!("mcp.claude_json"));
    println!("  \"mcpServers\": {{");
    println!("    \"pillbox\": {{");
    println!("      \"command\": \"node\",");
    println!("      \"args\": [\"{}/index.js\"]", mcp_dir.display());
    println!("    }}");
    println!("  }}\n");
    Ok(())
}

// ─── cmd_mcp_uninstall ───────────────────────────────────────────────────────

fn cmd_mcp_uninstall() -> Result<()> {
    let mcp_dir = pillbox::config::mcp_path().parent().unwrap().to_path_buf();
    if !mcp_dir.exists() {
        println!("{}\n", t!("mcp.not_installed"));
        return Ok(());
    }
    std::fs::remove_dir_all(&mcp_dir)?;
    println!("{} {}\n", "●".green().bold(), t!("mcp.uninstalled"));
    Ok(())
}

// ─── cmd_skill_install ───────────────────────────────────────────────────────

fn cmd_skill_install() -> Result<()> {
    let version = env!("CARGO_PKG_VERSION");
    let skill_path = pillbox::config::skill_path();
    let skill_dir = skill_path.parent().unwrap().to_path_buf();
    let url = format!(
        "https://github.com/kevinsillo/pillbox/releases/download/v{}/SKILL.md",
        version
    );

    std::fs::create_dir_all(&skill_dir)?;
    let pb = spinner(t!("skill.downloading"));
    let status = std::process::Command::new("curl")
        .args(["-fsSL", &url, "--output", skill_path.to_str().unwrap()])
        .status()?;
    pb.finish_and_clear();
    if !status.success() {
        anyhow::bail!("{}", t!("skill.error.download", url = url));
    }
    println!("{} {}\n", "●".green().bold(), t!("skill.installed", path = skill_path.display()));
    Ok(())
}

// ─── cmd_skill_uninstall ─────────────────────────────────────────────────────

fn cmd_skill_uninstall() -> Result<()> {
    let skill_dir = pillbox::config::skill_path()
        .parent()
        .unwrap()
        .to_path_buf();
    if !skill_dir.exists() {
        println!("{}\n", t!("skill.not_installed"));
        return Ok(());
    }
    std::fs::remove_dir_all(&skill_dir)?;
    println!("{} {}\n", "●".green().bold(), t!("skill.uninstalled"));
    Ok(())
}

// ─── cmd_uninstall ───────────────────────────────────────────────────────────

fn cmd_uninstall() -> Result<()> {
    use inquire::Confirm;

    println!("{}\n", t!("uninstall.title").bold());

    let mcp_dir = pillbox::config::mcp_path().parent().unwrap().to_path_buf();
    if mcp_dir.exists() {
        if Confirm::new(&t!("uninstall.mcp.prompt"))
            .with_default(false)
            .prompt()?
        {
            std::fs::remove_dir_all(&mcp_dir)?;
            println!("{} {}", "●".green().bold(), t!("uninstall.mcp.done"));
        }
    }

    let skill_dir = pillbox::config::skill_path()
        .parent()
        .unwrap()
        .to_path_buf();
    if skill_dir.exists() {
        if Confirm::new(&t!("uninstall.skill.prompt"))
            .with_default(false)
            .prompt()?
        {
            std::fs::remove_dir_all(&skill_dir)?;
            println!("{} {}", "●".green().bold(), t!("uninstall.skill.done"));
        }
    }

    let global_db = pillbox::config::global_db_path();
    if global_db.exists() {
        if Confirm::new(&t!("uninstall.db.prompt"))
            .with_default(false)
            .prompt()?
        {
            std::fs::remove_file(&global_db)?;
            println!("{} {}", "●".green().bold(), t!("uninstall.db.done"));
        }
    }

    let bin_path = std::env::current_exe()?;
    if Confirm::new(&t!("uninstall.bin.prompt", path = bin_path.display()))
        .with_default(false)
        .prompt()?
    {
        println!("{}\n", t!("uninstall.bin.manual", path = bin_path.display()));
    }

    println!();
    Ok(())
}

// ─── cmd_lang ────────────────────────────────────────────────────────────────

fn cmd_lang_show() -> Result<()> {
    let current = rust_i18n::locale().to_string();
    let rows: Vec<[String; 2]> = i18n::SUPPORTED
        .iter()
        .map(|(code, name)| {
            let label = if *code == current.as_str() {
                format!("{} {} {}", name, "●".green(), t!("lang.active").green())
            } else {
                name.to_string()
            };
            [code.to_string(), label]
        })
        .collect();

    let current_name = i18n::SUPPORTED
        .iter()
        .find(|(c, _)| *c == current.as_str())
        .map(|(_, n)| *n)
        .unwrap_or(&current);

    println!(
        "{}: {} ({})\n",
        t!("lang.current").bold(),
        current_name,
        current
    );
    println!("{}\n", output::table::dict(rows));
    Ok(())
}

fn cmd_lang_set(code: String) -> Result<()> {
    let code = code.to_lowercase();
    if !i18n::is_supported(&code) {
        let available = i18n::SUPPORTED
            .iter()
            .map(|(c, _)| *c)
            .collect::<Vec<_>>()
            .join(", ");
        anyhow::bail!("{}", t!("lang.set.invalid", lang = code, available = available));
    }
    i18n::save(&code)?;
    rust_i18n::set_locale(&code);
    println!("{} {}\n", "●".green().bold(), t!("lang.set.ok", lang = code));
    Ok(())
}

// ─── Helpers ─────────────────────────────────────────────────────────────────

fn open_resolved_db() -> Result<(rusqlite::Connection, std::path::PathBuf)> {
    let path = pillbox::config::resolve_db_path().ok_or_else(|| {
        anyhow::anyhow!("{}", t!("db.open_not_found"))
    })?;
    let conn = pillbox::db::connection::open(&path)?;
    Ok((conn, path))
}

fn find_current_bottle() -> Result<pillbox::domain::bottle::Bottle> {
    use pillbox::db::{connection, store::bottles};
    let global_path = pillbox::config::global_db_path();
    let conn = connection::open(&global_path)?;
    let current = std::env::current_dir()?;
    bottles::list(&conn)?
        .into_iter()
        .filter(|b| current.starts_with(&b.directory))
        .max_by_key(|b| b.directory.len())
        .ok_or_else(|| anyhow::anyhow!("{}", t!("bottle.error.not_found")))
}

fn spinner(msg: impl Into<std::borrow::Cow<'static, str>>) -> ProgressBar {
    let pb = ProgressBar::new_spinner();
    pb.enable_steady_tick(Duration::from_millis(80));
    pb.set_style(
        ProgressStyle::default_spinner()
            .tick_strings(&["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"])
            .template("{spinner} {msg}")
            .unwrap(),
    );
    pb.set_message(msg);
    pb
}
