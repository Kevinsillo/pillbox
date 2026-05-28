//! Punto de entrada del binario `pillbox`.
//!
//! Parsea los argumentos de la CLI con `clap`, detecta el idioma, abre la DB
//! resuelta y delega cada subcomando al módulo `cmd` correspondiente.

mod cli;
mod cmd;
mod help;
mod i18n;
mod mcp;
mod output;

use anyhow::Result;
use clap::Parser;

use crate::cli::{
    BottleCommand, CapsuleCommand, Cli, Command, LangCommand, McpCommand, MigrateCommand,
    PillCommand, PrescriptionCommand, ServeCommand, SkillCommand,
};
use crate::help::{render_help, render_nested_help, render_root_help};

rust_i18n::i18n!("locales", fallback = "en");

#[tokio::main]
async fn main() -> Result<()> {
    // Logs SIEMPRE a stderr — stdout queda exclusivamente para el protocolo MCP
    // (JSON-RPC line-delimited). Cualquier byte de tracing que se filtre por
    // stdout rompe al cliente MCP.
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(tracing::Level::WARN.into()),
        )
        .init();

    rust_i18n::set_locale(&i18n::detect());

    let cli = Cli::parse();

    if cli.init_global {
        let path = pillbox::config::global_db_path();
        // `init_global` siempre crea/abre la DB global → scope Global (no es ambiguo:
        // el flag toma el path directamente de `global_db_path()`, no de `resolve_db_path`).
        pillbox::db::connection::open(&path, pillbox::db::DbScope::Global)?;
        return Ok(());
    }

    if cli.help {
        return cmd_root_help();
    }

    match cli.command {
        None => cmd_root_help(),
        Some(Command::Status) => cmd::status::run(),
        Some(Command::Serve { cmd }) => match cmd {
            Some(ServeCommand::Run { port }) => cmd::serve::cmd_serve_run(port).await,
            Some(ServeCommand::Install { port }) => cmd::serve::cmd_serve_install(port),
            Some(ServeCommand::Uninstall) => cmd::serve::cmd_serve_uninstall(),
            Some(ServeCommand::Start) => cmd::serve::cmd_serve_start(),
            Some(ServeCommand::Stop) => cmd::serve::cmd_serve_stop(),
            Some(ServeCommand::Status) => cmd::serve::cmd_serve_status(),
            None => cmd::serve::cmd_serve_info(),
        },
        Some(Command::Bottle { cmd }) => match cmd {
            Some(BottleCommand::Init) => cmd::bottle::cmd_bottle_init(),
            Some(BottleCommand::Status) => cmd::bottle::cmd_bottle_status(),
            Some(BottleCommand::List { limit }) => cmd::bottle::cmd_bottle_list(limit),
            Some(BottleCommand::Migrate { subcommand }) => match subcommand {
                None => cmd::bottle::cmd_migrate_help(&render_nested_help("bottle", "migrate")),
                Some(MigrateCommand::Global) => cmd::bottle::cmd_migrate_global(),
                Some(MigrateCommand::Local) => cmd::bottle::cmd_migrate_local(),
            },
            Some(BottleCommand::Delete { slug }) => cmd::bottle::cmd_bottle_delete(&slug),
            Some(BottleCommand::Repair { slug }) => cmd::bottle::cmd_bottle_repair(&slug),
            Some(BottleCommand::Vinculate { directory }) => {
                cmd::bottle::cmd_bottle_vinculate(directory)
            }
            None => cmd_sub_help("bottle"),
        },
        Some(Command::Pill { cmd }) => match cmd {
            Some(PillCommand::Show { id }) => cmd::pill::cmd_pill_show(&id),
            None => cmd_sub_help("pill"),
        },
        Some(Command::Capsule { cmd }) => match cmd {
            Some(CapsuleCommand::List {
                limit,
                archived_limit,
            }) => cmd::capsule::cmd_capsule_list(limit, archived_limit),
            Some(CapsuleCommand::Show { id }) => cmd::capsule::cmd_capsule_show(&id),
            None => cmd_sub_help("capsule"),
        },
        Some(Command::Prescription { cmd }) => match cmd {
            Some(PrescriptionCommand::Open { title }) => {
                cmd::prescription::cmd_prescription_open(title)
            }
            Some(PrescriptionCommand::List {
                limit,
                archived_limit,
            }) => cmd::prescription::cmd_prescription_list(limit, archived_limit),
            Some(PrescriptionCommand::Show {
                id,
                limit,
                archived_limit,
            }) => cmd::prescription::cmd_prescription_show(id, limit, archived_limit),
            Some(PrescriptionCommand::Close { id }) => {
                cmd::prescription::cmd_prescription_close(id)
            }
            Some(PrescriptionCommand::Reopen { id }) => {
                cmd::prescription::cmd_prescription_reopen(id)
            }
            None => cmd_sub_help("prescription"),
        },
        Some(Command::Mcp { cmd }) => match cmd {
            Some(McpCommand::Install) => cmd::mcp::cmd_mcp_install(),
            Some(McpCommand::Uninstall) => cmd::mcp::cmd_mcp_uninstall(),
            Some(McpCommand::Run) => cmd::mcp::cmd_mcp_run(),
            None => cmd::mcp::cmd_mcp_status(),
        },
        Some(Command::Skill { cmd }) => match cmd {
            Some(SkillCommand::Install) => cmd::skill::cmd_skill_install(),
            Some(SkillCommand::Uninstall) => cmd::skill::cmd_skill_uninstall(),
            None => cmd::skill::cmd_skill_status(),
        },
        Some(Command::Lang { cmd }) => match cmd {
            Some(LangCommand::Set { code }) => cmd::lang::cmd_lang_set(code),
            None => cmd::lang::cmd_lang_show(&render_help("lang")),
        },
        Some(Command::Uninstall) => cmd::uninstall::run(),
        Some(Command::Help) => cmd_root_help(),
    }
}

/// Imprime el logo y la ayuda raíz en la salida estándar.
fn cmd_root_help() -> Result<()> {
    output::fmt::print_logo(env!("CARGO_PKG_VERSION"));
    println!("\n{}\n", render_root_help());
    Ok(())
}

/// Imprime la ayuda de un subcomando directamente en stdout.
fn cmd_sub_help(subcmd: &str) -> Result<()> {
    println!("\n{}\n", render_help(subcmd));
    Ok(())
}
