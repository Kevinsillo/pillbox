//! Definición de la CLI con `clap`: enums de subcomandos y árbol de comandos.
//!
//! `main.rs` actúa como dispatcher: parsea `Cli` y delega cada rama a su módulo
//! `cmd::*`. Las traducciones de la ayuda viven en `help.rs`.

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "pillbox",
    version,
    about = "Persistent knowledge memory for AI agents",
    disable_help_subcommand = true,
    disable_help_flag = true
)]
pub struct Cli {
    /// Inicializa la DB global (~/.pillbox/pillbox.db). Llamado por install.sh.
    #[arg(long, hide = true)]
    pub init_global: bool,

    /// Muestra esta ayuda.
    #[arg(short, long)]
    pub help: bool,

    #[command(subcommand)]
    pub command: Option<Command>,
}

#[derive(Subcommand)]
pub enum Command {
    /// Estado global: DBs, bottle activo, servidor, MCP y skill.
    Status,

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

    /// Operaciones sobre prescriptions del bottle actual.
    Prescription {
        #[command(subcommand)]
        cmd: Option<PrescriptionCommand>,
    },

    /// Operaciones sobre pills.
    Pill {
        #[command(subcommand)]
        cmd: Option<PillCommand>,
    },

    /// Operaciones sobre capsules.
    Capsule {
        #[command(subcommand)]
        cmd: Option<CapsuleCommand>,
    },

    /// Gestiona el servidor MCP.
    Mcp {
        #[command(subcommand)]
        cmd: Option<McpCommand>,
    },

    /// Gestiona la skill del agente IA.
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
    Help,
}

#[derive(Subcommand)]
pub enum ServeCommand {
    /// Ejecuta el servidor en primer plano (invocado por el gestor de servicios).
    #[command(hide = true)]
    Run {
        #[arg(short, long, default_value_t = pillbox::config::DEFAULT_PORT)]
        port: u16,
    },
    /// Instala el servidor HTTP como servicio del sistema.
    Install {
        #[arg(short, long, default_value_t = pillbox::config::DEFAULT_PORT)]
        port: u16,
    },
    /// Desinstala el servicio del sistema.
    Uninstall,
    /// Arranca el servicio del sistema.
    Start,
    /// Detiene el servicio del sistema.
    Stop,
    /// Muestra el estado del servicio.
    Status,
}

#[derive(Subcommand)]
pub enum BottleCommand {
    /// Inicializa un bottle en el directorio actual (wizard interactivo).
    Init,
    /// Estado del bottle del directorio actual.
    Status,
    /// Lista los bottles registrados en la DB global.
    List {
        #[arg(short, long, default_value = "20")]
        limit: u32,
    },
    /// Migra el bottle entre DB local y global.
    Migrate {
        #[command(subcommand)]
        subcommand: Option<MigrateCommand>,
    },
    /// Elimina un bottle del registro global (requiere confirmar el slug).
    Delete {
        /// Slug del bottle a eliminar.
        slug: String,
    },
    /// Corrige la ruta de un bottle desvinculado apuntando a su nueva ubicación.
    Repair {
        /// Slug del bottle a reparar.
        slug: String,
    },
    /// Vincula una DB local al registro global del usuario actual.
    Vinculate {
        #[arg(value_name = "DIRECTORY")]
        directory: Option<std::path::PathBuf>,
    },
}

#[derive(Subcommand)]
pub enum MigrateCommand {
    /// Mueve el bottle de este directorio a la DB global.
    Global,
    /// Elige un bottle de la DB global y muévelo aquí.
    Local,
}

#[derive(Subcommand)]
pub enum PrescriptionCommand {
    /// Abre una nueva prescripción para el bottle actual.
    Open {
        /// Título de la tarea o funcionalidad.
        title: String,
    },
    /// Lista las prescriptions del bottle actual (más recientes primero).
    List {
        #[arg(short, long, default_value = "10")]
        limit: u32,
        /// Límite de prescriptions archivadas mostradas (0 oculta la sección).
        #[arg(long, default_value_t = pillbox::config::ARCHIVED_LIMIT_DEFAULT)]
        archived_limit: u32,
    },
    /// Muestra el detalle de una prescription y sus pills.
    Show {
        /// ID (o prefijo) de la prescription.
        id: String,
        #[arg(short, long, default_value = "20")]
        limit: u32,
        /// Límite de pills archivadas mostradas (0 oculta la sección).
        #[arg(long, default_value_t = pillbox::config::ARCHIVED_LIMIT_DEFAULT)]
        archived_limit: u32,
    },
    /// Cierra una prescripción abierta del bottle actual.
    Close {
        /// ID (o prefijo) de la prescription a cerrar. Si se omite y solo hay
        /// una abierta, se cierra esa; si hay varias, se exige especificarla.
        id: Option<String>,
    },
    /// Reabre una prescription cerrada (limpia su `ended_at`).
    Reopen {
        /// ID (o prefijo ≥8 chars) de la prescription a reabrir.
        id: String,
    },
}

#[derive(Subcommand)]
pub enum PillCommand {
    /// Muestra el detalle de una pill por su UUID.
    Show {
        /// UUID de la pill (visible en `prescription show`).
        id: String,
    },
}

#[derive(Subcommand)]
pub enum CapsuleCommand {
    /// Lista las capsules globales (activas y archivadas).
    List {
        #[arg(short, long, default_value = "50")]
        limit: u32,
        /// Límite de capsules archivadas mostradas (0 oculta la sección).
        #[arg(long, default_value_t = pillbox::config::ARCHIVED_LIMIT_DEFAULT)]
        archived_limit: u32,
    },
    /// Muestra el detalle de una capsule por UUID.
    Show {
        /// UUID de la capsule.
        id: String,
    },
}

#[derive(Subcommand)]
pub enum McpCommand {
    /// Instala el servidor MCP en ~/.pillbox/mcp/.
    Install,
    /// Desinstala el servidor MCP.
    Uninstall {
        /// Proveedor destino (claude | opencode). Si se omite, se detecta o se pregunta.
        #[arg(long)]
        provider: Option<String>,
    },
    /// Arranca el loop persistente MCP (NDJSON sobre stdin/stdout).
    ///
    /// Invocado por el wrapper TS (`pillboxExec`) como subproceso singleton.
    /// No diseñado para uso interactivo.
    #[command(hide = true)]
    Run,
}

#[derive(Subcommand)]
pub enum SkillCommand {
    /// Instala las skills del agente IA en los proveedores detectados.
    Install,
    /// Desinstala la skill del agente IA.
    Uninstall {
        /// Proveedor destino (claude | opencode). Si se omite, se detecta o se pregunta.
        #[arg(long)]
        provider: Option<String>,
    },
}

#[derive(Subcommand)]
pub enum LangCommand {
    /// Cambia el idioma del CLI (es, en, de, it, pt, fr).
    Set {
        /// Código de idioma.
        code: String,
    },
}
