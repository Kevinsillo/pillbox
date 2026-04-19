mod cmd;
mod exec;
mod i18n;
mod output;
mod server;

use anyhow::Result;
use clap::{CommandFactory, Parser, Subcommand};
use owo_colors::OwoColorize;
use rust_i18n::t;

rust_i18n::i18n!("locales", fallback = "en");

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

    /// Cambia o muestra el idioma del CLI.
    Lang {
        #[command(subcommand)]
        cmd: Option<LangCommand>,
    },

    /// Desinstala componentes de Pillbox.
    Uninstall,

    /// Muestra esta ayuda.
    #[command(hide = true)]
    Help,
}

#[derive(Subcommand)]
enum ServeCommand {
    /// Arranca el servidor HTTP en segundo plano.
    Start {
        #[arg(short, long, default_value = "4242")]
        port: u16,
        /// Ejecuta en primer plano (para desarrollo).
        #[arg(short, long)]
        inline: bool,
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
        let path = pillbox::config::global_db_path();
        pillbox::db::connection::open(&path)?;
        return Ok(());
    }

    match cli.command {
        None => cmd_root_help(),
        Some(Command::Status) => cmd::status::run(),
        Some(Command::Exec) => exec::run(),
        Some(Command::Serve { cmd }) => match cmd {
            Some(ServeCommand::Start { port, inline }) => {
                cmd::serve::cmd_serve_start(port, inline).await
            }
            Some(ServeCommand::Stop) => cmd::serve::cmd_serve_stop(),
            Some(ServeCommand::Status) => cmd::serve::cmd_serve_status(),
            None => cmd_serve_info(),
        },
        Some(Command::Bottle { cmd }) => match cmd {
            Some(BottleCommand::Init) => cmd::bottle::cmd_bottle_init(),
            Some(BottleCommand::Status) => cmd::bottle::cmd_bottle_status(),
            Some(BottleCommand::List) => cmd::bottle::cmd_bottle_list(),
            Some(BottleCommand::Migrate { subcommand }) => match subcommand {
                None => cmd::bottle::cmd_migrate_help(),
                Some(MigrateCommand::Global) => cmd::bottle::cmd_migrate_global(),
                Some(MigrateCommand::Local) => cmd::bottle::cmd_migrate_local(),
            },
            None => cmd_sub_help("bottle"),
        },
        Some(Command::Pills { cmd }) => match cmd {
            Some(PillsCommand::List) => cmd::pills::cmd_pills_list(),
            None => cmd_sub_help("pills"),
        },
        Some(Command::Prescription { cmd }) => match cmd {
            Some(PrescriptionCommand::Open { title }) => {
                cmd::prescription::cmd_prescription_open(title)
            }
            Some(PrescriptionCommand::List { limit }) => {
                cmd::prescription::cmd_prescription_list(limit)
            }
            Some(PrescriptionCommand::Close) => cmd::prescription::cmd_prescription_close(),
            None => cmd_sub_help("prescription"),
        },
        Some(Command::Mcp { cmd }) => match cmd {
            Some(McpCommand::Install) => cmd::mcp::cmd_mcp_install(),
            Some(McpCommand::Uninstall) => cmd::mcp::cmd_mcp_uninstall(),
            None => cmd_mcp_status(),
        },
        Some(Command::Skill { cmd }) => match cmd {
            Some(SkillCommand::Install) => cmd::skill::cmd_skill_install(),
            Some(SkillCommand::Uninstall) => cmd::skill::cmd_skill_uninstall(),
            None => cmd_skill_status(),
        },
        Some(Command::Lang { cmd }) => match cmd {
            Some(LangCommand::Set { code }) => cmd::lang::cmd_lang_set(code),
            None => cmd::lang::cmd_lang_show(&render_help("lang")),
        },
        Some(Command::Uninstall) => cmd::uninstall::run(),
        Some(Command::Help) => cmd_root_help(),
    }
}

// ─── Helpers de ayuda (necesitan Cli::command()) ─────────────────────────────

fn t_help(key: &str, fallback: Option<String>) -> String {
    let translated = t!(key);
    if translated != key {
        translated.to_string()
    } else {
        fallback.unwrap_or_default()
    }
}

fn render_help_cmd(cmd: &mut clap::Command, name: &str) -> String {
    let mut out = String::new();
    let about_key = format!("help.about.{}", name);
    let about = t_help(&about_key, cmd.get_about().map(|a| a.to_string()));
    if !about.is_empty() {
        out.push_str(&format!("{}\n\n", about.bold()));
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
            let sub_key = format!("help.sub.{}.{}", name, s.get_name());
            let about = t_help(&sub_key, s.get_about().map(|a| a.to_string()));
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
    output::fmt::print_logo(env!("CARGO_PKG_VERSION"));
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
    let port = pillbox::config::DEFAULT_PORT;
    let pid = cmd::serve::read_pid(&pid_path);
    let running = cmd::serve::server_listening(port);
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
