mod exec;
mod server;

use anyhow::Result;
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "pillbox",
    version,
    about = "Persistent knowledge memory for AI agents"
)]
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

    /// Operaciones sobre el bottle del directorio actual.
    Bottle {
        #[command(subcommand)]
        cmd: BottleCommand,
    },
}

#[derive(Subcommand)]
enum BottleCommand {
    /// Migra el bottle actual entre DB local y global (upsert por sync_id).
    ///
    /// Por defecto copia de local → global. Con --reverse, de global → local.
    /// Usa --capsules para incluir también las capsules globales.
    Migrate {
        /// Invierte la dirección: global → local.
        #[arg(long)]
        reverse: bool,
        /// Incluye capsules (globales) en la migración.
        #[arg(long)]
        capsules: bool,
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
        Command::Status => cmd_status(),
        Command::Exec => exec::run(),
        Command::Serve { port } => cmd_serve(port).await,
        Command::Bottle { cmd } => match cmd {
            BottleCommand::Migrate { reverse, capsules } => cmd_bottle_migrate(reverse, capsules),
        },
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

fn cmd_bottle_migrate(reverse: bool, include_capsules: bool) -> Result<()> {
    use pillbox::db::{connection, migrate};

    let global_path = pillbox::config::global_db_path();
    let local_path = pillbox::config::local_db_path();

    if !global_path.exists() {
        anyhow::bail!("no se encontró la DB global ({})", global_path.display());
    }
    if !local_path.exists() {
        anyhow::bail!("no se encontró la DB local (.pillbox/pillbox.db) en el directorio actual");
    }

    let (src_path, dst_path) = if reverse {
        (&global_path, &local_path)
    } else {
        (&local_path, &global_path)
    };

    // Detectar el bottle por directorio en la DB de origen
    let src_conn = connection::open(src_path)?;
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
                "no hay ningún bottle registrado para '{}' en {}",
                dir_str,
                src_path.display()
            )
        })?;

    let direction = if reverse {
        "global → local"
    } else {
        "local → global"
    };
    println!("Migrando bottle '{}' ({})...", bottle_name, direction);

    let mut dst_conn = connection::open(dst_path)?;
    let result = migrate::migrate_bottle(&src_conn, &mut dst_conn, &bottle_name, include_capsules)?;

    println!("✓ Bottles:       {}", result.bottles);
    println!("✓ Prescripciones: {}", result.prescriptions);
    println!("✓ Pills:          {}", result.pills);
    if include_capsules {
        println!("✓ Capsules:       {}", result.capsules);
    }

    Ok(())
}
