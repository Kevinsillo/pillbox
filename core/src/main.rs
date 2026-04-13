mod exec;
mod server;

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
    /// Muestra la DB activa, versión de schema y conteo de entidades.
    Status,

    /// Lee un payload JSON de stdin, ejecuta la operación y escribe el resultado en stdout.
    /// Usado por el servidor MCP TypeScript como subproceso.
    Exec,

    /// Levanta el servidor HTTP en localhost:<puerto> (default: 4242).
    Serve {
        #[arg(short, long, default_value = "4242")]
        port: u16,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(tracing::Level::WARN.into()),
        )
        .init();

    let cli = Cli::parse();

    match cli.command {
        Command::Status      => cmd_status(),
        Command::Exec        => exec::run(),
        Command::Serve { port } => cmd_serve(port).await,
    }
}

fn cmd_status() -> Result<()> {
    match pillbox::config::resolve_db_path() {
        Some(path) => {
            println!("DB: {}", path.display());
            let conn = pillbox::db::connection::open(&path)?;
            let (schema_v, pill_count, capsule_count): (i64, i64, i64) = conn.query_row(
                "SELECT
                     (SELECT MAX(version) FROM schema_migrations),
                     (SELECT COUNT(*) FROM pills    WHERE deleted_at IS NULL),
                     (SELECT COUNT(*) FROM capsules WHERE deleted_at IS NULL)",
                [],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )?;
            println!("Schema:   v{}", schema_v);
            println!("Pills:    {}", pill_count);
            println!("Capsules: {}", capsule_count);
        }
        None => {
            println!("No se encontró ninguna Pillbox. Ejecuta el script de instalación.");
        }
    }
    Ok(())
}

async fn cmd_serve(port: u16) -> Result<()> {
    let path = pillbox::config::resolve_db_path()
        .ok_or_else(|| anyhow::anyhow!("no_db: no se encontró ninguna DB de Pillbox"))?;
    server::run(port, path).await
}
