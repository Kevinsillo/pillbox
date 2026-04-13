mod exec;
mod server;

use anyhow::Result;
use clap::{CommandFactory, Parser, Subcommand};
use indicatif::{ProgressBar, ProgressStyle};
use std::time::Duration;

// ─── CLI ──────────────────────────────────────────────────────────────────────

#[derive(Parser)]
#[command(name = "pillbox", version, about = "Persistent knowledge memory for AI agents")]
struct Cli {
    /// Inicializa la DB global (~/.pillbox/pillbox.db). Llamado por install.sh.
    #[arg(long, hide = true)]
    init_global: bool,

    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand)]
enum Command {
    /// Lista todos los bottles registrados en la DB global.
    List,

    /// Estado de la DB activa: schema, pills y capsules.
    Status,

    /// Diagnóstico del sistema: binario, DBs y bottle actual.
    Doctor,

    /// Lee JSON de stdin, ejecuta la operación y escribe JSON en stdout (usado por el MCP).
    #[command(hide = true)]
    Exec,

    /// Levanta el servidor HTTP (default: localhost:4242).
    Serve {
        #[arg(short, long, default_value = "4242")]
        port: u16,
    },

    /// Operaciones sobre el bottle del directorio actual.
    Bottle {
        #[command(subcommand)]
        cmd: BottleCommand,
    },

    /// Operaciones sobre prescriptions del bottle actual.
    Prescription {
        #[command(subcommand)]
        cmd: PrescriptionCommand,
    },
}

#[derive(Subcommand)]
enum BottleCommand {
    /// Inicializa un bottle en el directorio actual (wizard interactivo).
    Init,

    /// Estado del bottle del directorio actual.
    Status,

    /// Lista las pills más recientes del bottle actual.
    List {
        /// Máximo de pills a mostrar.
        #[arg(short, long, default_value = "20")]
        limit: u32,
    },

    /// Migra el bottle entre DB local y global (upsert por sync_id).
    Migrate {
        /// Invierte la dirección: global → local.
        #[arg(long)]
        reverse: bool,
        /// Incluye capsules globales en la migración.
        #[arg(long)]
        capsules: bool,
    },
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

// ─── Entry point ─────────────────────────────────────────────────────────────

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(tracing::Level::WARN.into()),
        )
        .init();

    let cli = Cli::parse();

    if cli.init_global {
        return cmd_init_global();
    }

    match cli.command {
        None => {
            Cli::command().print_help()?;
            println!();
            Ok(())
        }
        Some(Command::List) => cmd_list(),
        Some(Command::Status) => cmd_status(),
        Some(Command::Doctor) => cmd_doctor(),
        Some(Command::Exec) => exec::run(),
        Some(Command::Serve { port }) => cmd_serve(port).await,
        Some(Command::Bottle { cmd }) => match cmd {
            BottleCommand::Init => cmd_bottle_init(),
            BottleCommand::Status => cmd_bottle_status(),
            BottleCommand::List { limit } => cmd_bottle_list(limit),
            BottleCommand::Migrate { reverse, capsules } => cmd_bottle_migrate(reverse, capsules),
        },
        Some(Command::Prescription { cmd }) => match cmd {
            PrescriptionCommand::Open { title } => cmd_prescription_open(title),
            PrescriptionCommand::List { limit } => cmd_prescription_list(limit),
            PrescriptionCommand::Close => cmd_prescription_close(),
        },
    }
}

// ─── cmd_init_global ─────────────────────────────────────────────────────────

fn cmd_init_global() -> Result<()> {
    let path = pillbox::config::global_db_path();
    pillbox::db::connection::open(&path)?;
    Ok(())
}

// ─── cmd_list ────────────────────────────────────────────────────────────────

fn cmd_list() -> Result<()> {
    use pillbox::db::{connection, store::bottles};

    let path = pillbox::config::global_db_path();
    if !path.exists() {
        println!("No se encontró la DB global. Ejecuta el script de instalación.");
        return Ok(());
    }

    let conn = connection::open(&path)?;
    let all = bottles::list(&conn)?;

    if all.is_empty() {
        println!("No hay bottles registrados.");
        return Ok(());
    }

    println!("Bottles registrados ({})", all.len());
    println!("{}", "─".repeat(72));
    println!("{:<4} {:<22} {:<22} {}", "#", "Nombre", "Display", "Directorio");
    println!("{}", "─".repeat(72));
    for (i, b) in all.iter().enumerate() {
        println!(
            "{:<4} {:<22} {:<22} {}",
            i + 1,
            truncate(&b.name, 20),
            truncate(&b.display_name, 20),
            b.directory,
        );
    }
    println!("{}", "─".repeat(72));
    Ok(())
}

// ─── cmd_status ──────────────────────────────────────────────────────────────

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
        None => println!("No se encontró ninguna Pillbox. Ejecuta el script de instalación."),
    }
    Ok(())
}

// ─── cmd_doctor ──────────────────────────────────────────────────────────────

fn cmd_doctor() -> Result<()> {
    let bin_path = std::env::current_exe()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|_| "(desconocido)".into());

    println!("Pillbox Doctor");
    println!("{}", "═".repeat(52));
    println!();
    println!("Binario      {}", bin_path);
    println!();

    // DB global
    let global = pillbox::config::global_db_path();
    print!("DB global    {}", global.display());
    if global.exists() {
        match pillbox::db::connection::open(&global) {
            Ok(conn) => {
                let (v, btl, pills, caps): (i64, i64, i64, i64) = conn.query_row(
                    "SELECT
                         (SELECT MAX(version) FROM schema_migrations),
                         (SELECT COUNT(*) FROM bottles),
                         (SELECT COUNT(*) FROM pills    WHERE deleted_at IS NULL),
                         (SELECT COUNT(*) FROM capsules WHERE deleted_at IS NULL)",
                    [],
                    |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
                )?;
                println!();
                println!(
                    "             ✓ Schema v{}  —  {} bottles  —  {} pills  —  {} capsules",
                    v, btl, pills, caps
                );
            }
            Err(e) => println!("\n             ✗ Error al abrir: {}", e),
        }
    } else {
        println!("\n             ✗ No existe (ejecuta el script de instalación)");
    }
    println!();

    // DB local
    let local = pillbox::config::local_db_path();
    print!("DB local     {}", local.display());
    if local.exists() {
        match pillbox::db::connection::open(&local) {
            Ok(conn) => {
                let (v, pills): (i64, i64) = conn.query_row(
                    "SELECT
                         (SELECT MAX(version) FROM schema_migrations),
                         (SELECT COUNT(*) FROM pills WHERE deleted_at IS NULL)",
                    [],
                    |r| Ok((r.get(0)?, r.get(1)?)),
                )?;
                println!();
                println!("             ✓ Schema v{}  —  {} pills", v, pills);
            }
            Err(e) => println!("\n             ✗ Error al abrir: {}", e),
        }
    } else {
        println!("  (no existe en este directorio)");
    }
    println!();

    // Bottle actual
    print!("Bottle       ");
    if let Some(db_path) = pillbox::config::resolve_db_path() {
        if let Ok(conn) = pillbox::db::connection::open(&db_path) {
            let dir = std::env::current_dir()?.to_string_lossy().to_string();
            match pillbox::db::store::bottles::find_by_directory(&conn, &dir) {
                Ok(Some(b)) => {
                    println!("{} — \"{}\" — {}", b.name, b.display_name, b.scope);
                    let open_rx: Option<String> = conn
                        .query_row(
                            "SELECT title FROM prescriptions
                             WHERE bottle_id = ?1 AND ended_at IS NULL AND deleted_at IS NULL
                             LIMIT 1",
                            rusqlite::params![b.id],
                            |r| r.get(0),
                        )
                        .ok();
                    if let Some(title) = open_rx {
                        println!("             Prescripción abierta: \"{}\"", title);
                    }
                }
                _ => println!("✗ No hay bottle para este directorio"),
            }
        } else {
            println!("✗ No se pudo abrir la DB");
        }
    } else {
        println!("✗ No se encontró ninguna DB");
    }
    println!();

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

    println!("Inicializando bottle en {}...\n", dir_str);

    // Display name
    let display_name = Text::new("¿Cómo quieres llamar a este proyecto?")
        .with_default(&dir_basename)
        .prompt()?;

    let name = pillbox::normalize::bottle_name(&display_name);

    // Scope
    let scope_options = vec![
        "local  — .pillbox/pillbox.db (solo este proyecto)",
        "global — ~/.pillbox/pillbox.db (compartido)",
    ];
    let scope_choice = Select::new("¿Dónde guardar las memories?", scope_options).prompt()?;
    let scope = if scope_choice.starts_with("local") {
        BottleScope::Local
    } else {
        BottleScope::Global
    };

    let db_path = match &scope {
        BottleScope::Local => pillbox::config::local_db_path(),
        BottleScope::Global => pillbox::config::global_db_path(),
    };

    // Crear DB (connection::open crea el directorio y corre migraciones)
    let pb = spinner("Creando DB...");
    let mut conn = connection::open(&db_path)?;

    // Verificar que no existe ya un bottle para este directorio
    if bottles::find_by_directory(&conn, &dir_str)?.is_some() {
        pb.finish_and_clear();
        anyhow::bail!(
            "Ya existe un bottle registrado para este directorio en {}",
            db_path.display()
        );
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

    println!("✓ Bottle '{}' creado.", bottle.name);
    println!();
    println!("  Slug:    {}", bottle.name);
    println!("  Display: {}", bottle.display_name);
    println!("  DB:      {}", db_path.display());
    println!();

    // Si es local: preguntar gitignore y registrar en global
    if scope == BottleScope::Local {
        if current_dir.join(".git").exists() {
            let add_gi = Confirm::new("¿Añadir .pillbox/ a .gitignore?")
                .with_default(true)
                .prompt()
                .unwrap_or(false);
            if add_gi {
                add_to_gitignore(&current_dir)?;
                println!("✓ .gitignore actualizado.");
            }
        }

        let global_path = pillbox::config::global_db_path();
        if global_path.exists() {
            let pb2 = spinner("Registrando en DB global...");
            match register_in_global(&global_path, &name, &display_name, &dir_str, &scope) {
                Ok(_) => pb2.finish_with_message("✓ Registrado en DB global."),
                Err(e) => {
                    pb2.finish_and_clear();
                    eprintln!("⚠  No se pudo registrar en DB global: {}", e);
                }
            }
        }
    }

    println!();
    println!("Listo. Usa 'pillbox bottle status' para ver el estado.");
    Ok(())
}

fn add_to_gitignore(dir: &std::path::Path) -> Result<()> {
    use std::io::Write;
    let gi_path = dir.join(".gitignore");
    let entry = ".pillbox/\n";

    if gi_path.exists() {
        let content = std::fs::read_to_string(&gi_path)?;
        if content.lines().any(|l| l.trim() == ".pillbox/" || l.trim() == ".pillbox") {
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
    let bottle = find_current_bottle(&conn)?;

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

    println!("Bottle:  {} — \"{}\"", bottle.name, bottle.display_name);
    println!("Scope:   {}", bottle.scope);
    println!("Dir:     {}", bottle.directory);
    println!("Pills:   {}", pill_count);
    if let Some((id, title)) = open_rx {
        println!("Rx:      \"{}\" (abierta, id={})", title, &id[..id.len().min(8)]);
    } else {
        println!("Rx:      ninguna abierta");
    }
    Ok(())
}

// ─── cmd_bottle_list ─────────────────────────────────────────────────────────

fn cmd_bottle_list(limit: u32) -> Result<()> {
    let (conn, _) = open_resolved_db()?;
    let bottle = find_current_bottle(&conn)?;

    let mut stmt = conn.prepare(
        "SELECT p.id, p.compound, p.title, p.created_at
         FROM pills p
         JOIN prescriptions rx ON rx.id = p.prescription_id
         WHERE rx.bottle_id = ?1 AND p.deleted_at IS NULL
         ORDER BY p.created_at DESC
         LIMIT ?2",
    )?;

    let pills: Vec<(i64, String, String, String)> = stmt
        .query_map(rusqlite::params![bottle.id, limit], |r| {
            Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?))
        })?
        .filter_map(|r| r.ok())
        .collect();

    println!("Pills de '{}' (últimas {})", bottle.name, limit);
    if pills.is_empty() {
        println!("  (ninguna)");
        return Ok(());
    }
    println!("{}", "─".repeat(68));
    println!("{:<5} {:<16} {}", "#", "Compound", "Título");
    println!("{}", "─".repeat(68));
    for (i, (_, compound, title, _)) in pills.iter().enumerate() {
        println!(
            "{:<5} {:<16} {}",
            i + 1,
            truncate(compound, 14),
            truncate(&title, 46)
        );
    }
    println!("{}", "─".repeat(68));
    Ok(())
}

// ─── cmd_prescription_open ───────────────────────────────────────────────────

fn cmd_prescription_open(title: String) -> Result<()> {
    use pillbox::db::store::{prescriptions, PrescriptionAlreadyOpen};
    use pillbox::domain::prescription::NewPrescription;

    let (mut conn, _) = open_resolved_db()?;
    let bottle = find_current_bottle(&conn)?;

    match prescriptions::open(&mut conn, &NewPrescription { bottle_id: bottle.id, title }) {
        Ok(rx) => {
            println!("✓ Prescripción abierta: \"{}\"", rx.title);
            println!("  ID: {}", rx.id);
        }
        Err(e) => {
            if let Some(already) = e.downcast_ref::<PrescriptionAlreadyOpen>() {
                anyhow::bail!(
                    "Ya hay una prescripción abierta: \"{}\" ({} pills, id={}).\n\
                     Ciérrala con 'pillbox prescription close' antes de abrir una nueva.",
                    already.title,
                    already.pill_count,
                    &already.id[..already.id.len().min(8)],
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
    let bottle = find_current_bottle(&conn)?;
    let rxs = prescriptions::list_by_bottle(&conn, bottle.id, limit)?;

    println!("Prescriptions de '{}' (últimas {})", bottle.name, limit);
    if rxs.is_empty() {
        println!("  (ninguna)");
        return Ok(());
    }
    println!("{}", "─".repeat(68));
    println!("{:<10} {:<40} {}", "ID", "Título", "Estado");
    println!("{}", "─".repeat(68));
    for rx in &rxs {
        let short_id = &rx.id[..rx.id.len().min(8)];
        let estado = if rx.ended_at.is_some() { "cerrada" } else { "abierta" };
        println!("{:<10} {:<40} {}", short_id, truncate(&rx.title, 38), estado);
    }
    println!("{}", "─".repeat(68));
    Ok(())
}

// ─── cmd_prescription_close ──────────────────────────────────────────────────

fn cmd_prescription_close() -> Result<()> {
    use pillbox::db::store::prescriptions;

    let (mut conn, _) = open_resolved_db()?;
    let bottle = find_current_bottle(&conn)?;

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
        anyhow::anyhow!(
            "No hay ninguna prescripción abierta para el bottle '{}'.",
            bottle.name
        )
    })?;

    prescriptions::close(&mut conn, &rx_id)?;
    println!("✓ Prescripción cerrada: \"{}\"", rx_title);
    Ok(())
}

// ─── cmd_serve ───────────────────────────────────────────────────────────────

async fn cmd_serve(port: u16) -> Result<()> {
    let path = pillbox::config::resolve_db_path()
        .ok_or_else(|| anyhow::anyhow!("no_db: no se encontró ninguna DB de Pillbox"))?;
    server::run(port, path).await
}

// ─── cmd_bottle_migrate ──────────────────────────────────────────────────────

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

    let direction = if reverse { "global → local" } else { "local → global" };
    println!("Migrando bottle '{}' ({})...", bottle_name, direction);

    let mut dst_conn = connection::open(dst_path)?;
    let result = migrate::migrate_bottle(&src_conn, &mut dst_conn, &bottle_name, include_capsules)?;

    println!("✓ Bottles:        {}", result.bottles);
    println!("✓ Prescripciones: {}", result.prescriptions);
    println!("✓ Pills:          {}", result.pills);
    if include_capsules {
        println!("✓ Capsules:       {}", result.capsules);
    }
    Ok(())
}

// ─── Helpers ─────────────────────────────────────────────────────────────────

fn open_resolved_db() -> Result<(rusqlite::Connection, std::path::PathBuf)> {
    let path = pillbox::config::resolve_db_path().ok_or_else(|| {
        anyhow::anyhow!(
            "No se encontró ninguna DB de Pillbox.\n\
             Ejecuta 'pillbox bottle init' para crear una."
        )
    })?;
    let conn = pillbox::db::connection::open(&path)?;
    Ok((conn, path))
}

fn find_current_bottle(
    conn: &rusqlite::Connection,
) -> Result<pillbox::domain::bottle::Bottle> {
    let dir = std::env::current_dir()?.to_string_lossy().to_string();
    pillbox::db::store::bottles::find_by_directory(conn, &dir)?.ok_or_else(|| {
        anyhow::anyhow!(
            "No hay ningún bottle para este directorio.\n\
             Ejecuta 'pillbox bottle init' para crear uno."
        )
    })
}

fn truncate(s: &str, max: usize) -> String {
    let chars: Vec<char> = s.chars().collect();
    if chars.len() <= max {
        s.to_string()
    } else {
        let t: String = chars[..max.saturating_sub(1)].iter().collect();
        format!("{}…", t)
    }
}

fn spinner(msg: &'static str) -> ProgressBar {
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
