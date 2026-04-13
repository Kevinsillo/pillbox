use anyhow::Result;
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "pillbox", version, about = "Persistent knowledge memory for AI agents")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Muestra la ruta de la DB activa y el estado del schema.
    Status,
}

fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(tracing::Level::WARN.into()),
        )
        .init();

    let cli = Cli::parse();

    match cli.command {
        Command::Status => cmd_status(),
    }
}

fn cmd_status() -> Result<()> {
    match pillbox::config::resolve_db_path() {
        Some(path) => {
            println!("DB: {}", path.display());
            let conn = pillbox::db::connection::open(&path)?;
            let version: i64 = conn.query_row(
                "SELECT MAX(version) FROM schema_migrations",
                [],
                |r| r.get(0),
            )?;
            println!("Schema: v{}", version);
        }
        None => {
            println!("No se encontró ninguna Pillbox. Ejecuta el script de instalación.");
        }
    }
    Ok(())
}
