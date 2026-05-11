//! Funciones de formato y presentación de entidades del dominio en terminal.

use super::{table, truncate};
use owo_colors::OwoColorize;
use pillbox::domain::{bottle::Bottle, capsule::Capsule, pill::Pill, prescription::Prescription};
use rust_i18n::t;

// ─── Logo ─────────────────────────────────────────────────────────────────────

const LOGO: [&str; 7] = [
    "████████              ████████",
    "███         ██████         ███",
    "███      ████████████      ███",
    "███      ▓▓▓▓▓▓▓▓▓▓▓▓      ███",
    "███      ▒▒▒▒▒▒▒▒▒▒▒▒      ███",
    "███         ▒▒▒▒▒▒         ███",
    "████████              ████████",
];

fn color_line(line: &str) -> String {
    let mut out = String::new();
    let mut seg = String::new();
    let mut red = false;
    for ch in line.chars() {
        let is_red = matches!(ch, '▓' | '▒');
        if is_red != red && !seg.is_empty() {
            out.push_str(&if red {
                seg.red().to_string()
            } else {
                seg.bright_white().to_string()
            });
            seg.clear();
        }
        red = is_red;
        seg.push(ch);
    }
    if !seg.is_empty() {
        out.push_str(&if red {
            seg.red().to_string()
        } else {
            seg.bright_white().to_string()
        });
    }
    out
}

/// Imprime el logo ASCII de pillbox junto con la versión del binario.
pub fn print_logo(version: &str) {
    for (i, line) in LOGO.iter().enumerate() {
        if i == 6 {
            println!(" {}  pillbox {}", color_line(line), version.dimmed());
        } else {
            println!(" {}", color_line(line));
        }
    }
    println!();
}

// ─── Pattern helpers ──────────────────────────────────────────────────────────

/// Pattern A — confirmation with inline key-value details.
fn print_a(action: &str, details: &[(&str, &str)]) {
    let key_width = details.iter().map(|(k, _)| k.len()).max().unwrap_or(0) + 1;
    println!("\n{} {}", "✓".green().bold(), action);
    for (key, val) in details {
        println!("   {:<width$}│  {}", key.dimmed(), val, width = key_width);
    }
    println!();
}

/// Pattern B — simple one-line confirmation.
fn print_b(action: &str) {
    println!("\n{} {}\n", "✓".green().bold(), action);
}

// ─── Bottles ──────────────────────────────────────────────────────────────────

/// Fila de datos para la tabla de listing de bottles registrados.
pub struct BottleListRow {
    pub name: String,
    pub directory: String,
    pub scope: String,
    pub linked: bool,
    pub is_active: bool,
}

/// Muestra la lista de bottles registrados con su estado de enlace.
pub fn bottles_registered_list(rows: &[BottleListRow], total: u32) {
    let count = rows.len();
    println!("\nBottles    {}", count.to_string().bold());

    if rows.is_empty() {
        println!("\n  {}\n", t!("bottles.none").dimmed());
        return;
    }

    println!();
    let table_rows = rows
        .iter()
        .map(|r| {
            let estado = if !r.linked {
                "✗".red().to_string()
            } else if r.is_active {
                "●".green().to_string()
            } else {
                "○".dimmed().to_string()
            };

            let name_cell = if r.linked {
                r.name.clone()
            } else {
                r.name.dimmed().to_string()
            };

            let dir_cell = if r.linked {
                truncate(&r.directory, 50)
            } else {
                format!(
                    "{}  {}",
                    truncate(&r.directory, 40),
                    t!("bottles.unlinked").red()
                )
            };

            let scope_cell = r.scope.dimmed().to_string();

            vec![estado, name_cell, dir_cell, scope_cell]
        })
        .collect();

    print!(
        "{}",
        table::plain_list(
            &[
                " ",
                t!("bottles.list.col.name").as_ref(),
                t!("bottles.list.col.dir").as_ref(),
                t!("bottles.list.col.scope").as_ref(),
            ],
            table_rows,
        )
    );
    let hidden = total.saturating_sub(rows.len() as u32);
    if hidden > 0 {
        println!("   … {} más", hidden);
    } else {
        println!();
    }
    println!();
}

/// Muestra el estado actual de un bottle: nombre, scope, directorio, pills y prescripción abierta.
pub fn bottle_status(bottle: &Bottle, pill_count: i64, open_rx: Option<(String, String)>) {
    let rx_val = match open_rx {
        Some((id, title)) => format!(
            "{}",
            t!(
                "bottle.status.rx_open",
                title = title,
                id = &id[..id.len().min(8)]
            )
        ),
        None => t!("bottle.status.rx_none").dimmed().to_string(),
    };
    println!(
        "\n{}",
        table::dict(vec![
            [
                t!("bottle.status.labels.bottle").bold().to_string(),
                format!("{} — \"{}\"", bottle.name, bottle.display_name),
            ],
            [
                t!("bottle.status.labels.scope").bold().to_string(),
                bottle.scope.to_string(),
            ],
            [
                t!("bottle.status.labels.dir").bold().to_string(),
                bottle.directory.clone(),
            ],
        ])
    );
    println!(
        "\n{}\n",
        table::dict(vec![
            [
                t!("bottle.status.labels.pills").bold().to_string(),
                pill_count.to_string(),
            ],
            [t!("bottle.status.labels.rx").bold().to_string(), rx_val],
        ])
    );
}

// ─── Status ───────────────────────────────────────────────────────────────────

/// Datos de una DB (global o local) para mostrar en el panel de estado.
pub struct StatusDb {
    pub path: String,
    pub result: Option<Result<(i64, i64, i64, i64, i64), String>>,
}

/// Información del bottle activo para mostrar en el panel de estado.
pub struct StatusBottle {
    pub open_rx: Option<String>,
}

/// Muestra el panel de estado completo del sistema: binario, DBs, servidor, MCP y skill.
pub fn status(
    bin_path: &str,
    global: StatusDb,
    local: StatusDb,
    bottle: Option<StatusBottle>,
    server_port: Option<u16>,
    mcp_path: &std::path::Path,
    skill_path: &std::path::Path,
) {
    let db_val = |s: StatusDb, bottle: Option<StatusBottle>, is_local: bool| -> String {
        let mut val = match s.result {
            None => return format!("{}\n{}", s.path, t!("status.db.none").dimmed()),
            Some(Ok((_v, _btl, pills, _caps, rxs))) if is_local => format!(
                "{}\n{}  {}{}  {}{}",
                s.path,
                "●".green(),
                t!("status.db.pills").bold(),
                pills.to_string().green(),
                t!("status.db.prescriptions").bold(),
                rxs.to_string().green(),
            ),
            Some(Ok((v, btl, _pills, caps, _rxs))) => format!(
                "{}\n{}  {}{}  {}{}  {}{}",
                s.path,
                "●".green(),
                t!("status.db.schema").bold(),
                format!("v{}", v).green(),
                t!("status.db.bottles").bold(),
                btl.to_string().green(),
                t!("status.db.capsules").bold(),
                caps.to_string().green(),
            ),
            Some(Err(e)) => format!("{}\n{} {}", s.path, "●".red(), e),
        };
        if let Some(b) = bottle {
            if let Some(title) = b.open_rx {
                val.push_str(&format!(
                    "\n{}  \"{}\"",
                    t!("status.db.rx").bold(),
                    title.green()
                ));
            }
        }
        val
    };

    let server_val = match server_port {
        Some(port) => format!(
            "{} {}",
            "●".green(),
            t!("status.server.running", port = port)
        ),
        None => format!("{} {}", "●".red(), t!("status.server.stopped")),
    };

    let mcp_val = if mcp_path.exists() {
        format!("{} {}", "●".green(), mcp_path.display())
    } else {
        format!("{} {}", "●".red(), t!("status.component.missing"))
    };

    let skill_val = if skill_path.exists() {
        format!("{} {}", "●".green(), skill_path.display())
    } else {
        format!("{} {}", "●".red(), t!("status.component.missing"))
    };

    println!(
        "\n{}",
        table::dict(vec![[
            t!("status.labels.bin").bold().to_string(),
            bin_path.to_string(),
        ]])
    );
    println!(
        "\n{}",
        table::dict(vec![
            [
                t!("status.labels.global").bold().to_string(),
                db_val(global, None, false),
            ],
            [
                t!("status.labels.local").bold().to_string(),
                db_val(local, bottle, true),
            ],
        ])
    );
    println!(
        "\n{}\n",
        table::dict(vec![
            [t!("status.labels.web").bold().to_string(), server_val],
            [t!("status.labels.mcp").bold().to_string(), mcp_val],
            [t!("status.labels.skill").bold().to_string(), skill_val],
        ])
    );
}

/// Confirma en pantalla que el servicio ha arrancado (Pattern A).
pub fn serve_started(url: &str) {
    print_a(
        &t!("serve.start.success"),
        &[(t!("serve.labels.url").as_ref(), &url.cyan().to_string())],
    );
}

/// Confirma en pantalla que el servicio se ha detenido (Pattern B).
pub fn serve_stopped() {
    print_b(&t!("serve.stop.success"));
}

/// Muestra el estado del servicio del sistema: running/stopped y URL canónica (Pattern D).
///
/// Cuando el servicio está parado se omite la URL.
pub fn serve_status(running: bool, url: Option<&str>) {
    let estado = if running {
        format!("{} {}", "●".green(), t!("serve.running"))
    } else {
        format!("{} {}", "●".red(), t!("serve.stopped"))
    };
    let mut rows = vec![[t!("serve.labels.estado").bold().to_string(), estado]];
    if let Some(u) = url {
        rows.push([
            t!("serve.labels.url").bold().to_string(),
            u.cyan().to_string(),
        ]);
    }
    println!("\n{}\n", table::dict(rows));
}

/// Muestra un texto de ayuda con una línea de estado insertada tras el about.
/// `status_line` debe incluir label y valor ya formateados (ej. "Estado: ● path").
pub fn help_with_status(status_line: &str, help: &str) {
    let split = help.find("\n\n").unwrap_or(help.len());
    let (title, rest) = help.split_at(split);
    println!("\n{}\n\n{}{}\n", title, status_line, rest);
}

/// Muestra el estado de un componente (MCP o skill) junto con su texto de ayuda.
pub fn component_status_with_help(path: &std::path::Path, help: &str) {
    let status = if path.exists() {
        format!("{} {}", "●".green(), path.display())
    } else {
        format!("{} {}", "●".red(), t!("status.component.missing"))
    };
    let line = format!("{}: {}", t!("status.component.estado").bold(), status);
    help_with_status(&line, help);
}

/// Imprime en stderr el mensaje de error cuando no se encuentra la DB local.
pub fn db_not_found() {
    eprintln!("\n{} {}\n", "✗".red().bold(), t!("db.not_found"));
}

// ─── Prescriptions ────────────────────────────────────────────────────────────

/// Muestra la lista de prescripciones de un bottle.
///
/// Caller pre-divide los registros entre `active` y `archived`. La función
/// aplica el cap visible (`archived_limit`) sobre los archivados y muestra
/// el trailer `... N más archivados` con `N = archived_total - archived_limit`
/// cuando el total exceda el cap. Si `archived_limit == 0`, oculta toda la
/// sección de archivadas (sin header, sin filas, sin trailer).
pub fn prescriptions_list(
    bottle_name: &str,
    db_path: &str,
    active: &[Prescription],
    archived: &[Prescription],
    archived_limit: u32,
    archived_total: u32,
    total: u32,
) {
    let count = active.len();
    println!(
        "\nPrescriptions  {}  {}    {}",
        "·".dimmed(),
        bottle_name.dimmed(),
        count.to_string().bold()
    );
    println!("  {}", db_path.dimmed());

    let archived_cap = archived_limit as usize;
    let archived_shown: Vec<&Prescription> = if archived_cap == 0 {
        Vec::new()
    } else {
        archived.iter().take(archived_cap).collect()
    };

    if active.is_empty() && archived_shown.is_empty() {
        println!("\n  {}\n", t!("prescriptions.none").dimmed());
        return;
    }

    if !active.is_empty() {
        println!();
        let rows = active
            .iter()
            .map(|rx| {
                let short_id = &rx.id[..rx.id.len().min(8)];
                let estado = if rx.ended_at.is_some() {
                    t!("prescriptions.state.closed").dimmed().to_string()
                } else {
                    t!("prescriptions.state.open").green().to_string()
                };
                let author = match &rx.author_name {
                    Some(name) => truncate(name, 18),
                    None => "-".dimmed().to_string(),
                };
                vec![short_id.to_string(), truncate(&rx.title, 40), estado, author]
            })
            .collect();
        print!(
            "{}",
            table::plain_list(
                &[
                    t!("prescriptions.list.col.id").as_ref(),
                    t!("prescriptions.list.col.title").as_ref(),
                    t!("prescriptions.list.col.state").as_ref(),
                    t!("prescriptions.list.col.author").as_ref(),
                ],
                rows
            )
        );
        let active_count = active.len() as u32;
        let hidden = total.saturating_sub(active_count + archived_total);
        if hidden > 0 {
            println!("   … {} más", hidden);
        } else {
            println!();
        }
    }

    if !archived_shown.is_empty() {
        let n = archived_shown.len();
        println!(
            "\n  {}\n",
            t!("prescriptions.list.archived_section", count = n)
                .dimmed()
                .to_string()
        );
        let rows = archived_shown
            .iter()
            .map(|rx| {
                let short_id = &rx.id[..rx.id.len().min(8)];
                let estado = if rx.ended_at.is_some() {
                    t!("prescriptions.state.closed").dimmed().to_string()
                } else {
                    t!("prescriptions.state.open").dimmed().to_string()
                };
                let archived_date = rx
                    .deleted_at
                    .as_deref()
                    .and_then(|d| d.get(..10))
                    .unwrap_or("—")
                    .dimmed()
                    .to_string();
                let author = match &rx.author_name {
                    Some(name) => truncate(name, 18).dimmed().to_string(),
                    None => "-".dimmed().to_string(),
                };
                vec![
                    short_id.dimmed().to_string(),
                    truncate(&rx.title, 36).dimmed().to_string(),
                    estado,
                    author,
                    archived_date,
                ]
            })
            .collect();
        print!(
            "{}",
            table::plain_list(
                &[
                    t!("prescriptions.list.col.id").as_ref(),
                    t!("prescriptions.list.col.title").as_ref(),
                    t!("prescriptions.list.col.state").as_ref(),
                    t!("prescriptions.list.col.author").as_ref(),
                    t!("prescriptions.list.col.archived_at").as_ref(),
                ],
                rows
            )
        );
        let archived_hidden = archived_total.saturating_sub(archived_cap as u32);
        if archived_hidden > 0 {
            println!(
                "   {}",
                t!("prescriptions.list.more_archived", count = archived_hidden)
                    .dimmed()
            );
        }
        println!();
    } else {
        println!();
    }
}

/// Confirma en pantalla la apertura de una prescripción nueva.
pub fn prescription_opened(id: &str, title: &str) {
    let short_id = &id[..id.len().min(8)];
    print_a(
        &t!("prescriptions.msg.opened"),
        &[("title", title), ("id", short_id)],
    );
}

/// Confirma en pantalla el cierre de una prescripción.
pub fn prescription_closed(_title: &str) {
    print_b(&t!("prescriptions.msg.closed"));
}

// ─── Bottle init ──────────────────────────────────────────────────────────────

/// Informa al usuario del inicio del proceso de inicialización de un bottle.
pub fn bottle_init_start(dir: &str) {
    println!("\n{}\n", t!("bottle.init.start", dir = dir));
}

/// Confirma la creación del bottle mostrando su slug, nombre visible y ruta de la DB.
pub fn bottle_init_created(name: &str, display_name: &str, db_path: &std::path::Path) {
    let db_str = db_path.display().to_string();
    let action = t!("bottle.init.created", name = name).to_string();
    let slug_label = t!("bottle.init.col.slug").to_string();
    let display_label = t!("bottle.init.col.display").to_string();
    let db_label = t!("bottle.init.col.db").to_string();
    print_a(
        &action,
        &[
            (&slug_label, name),
            (&display_label, display_name),
            (&db_label, &db_str),
        ],
    );
}

/// Confirma que `.pillbox/` fue añadido al `.gitignore` del proyecto.
pub fn bottle_init_gitignore() {
    print_b(&t!("bottle.init.gitignore.done"));
}

/// Imprime el mensaje de finalización del flujo de inicialización del bottle.
pub fn bottle_init_done() {
    println!("   {}  {}\n", "→".dimmed(), t!("bottle.init.done").dimmed());
}

// ─── Bottle vinculate ─────────────────────────────────────────────────────────

/// Confirma que la DB local fue vinculada al registro global.
pub fn bottle_vinculate_done(name: &str, path: &str) {
    print_b(&t!("bottle.vinculate.done", name = name, path = path));
}

/// Informa que la DB local ya estaba vinculada al registro global.
pub fn bottle_vinculate_already(name: &str, path: &str) {
    println!(
        "\n{} {}\n",
        "→".dimmed(),
        t!("bottle.vinculate.already_linked", name = name, path = path).dimmed()
    );
}

// ─── MCP / Skill install ──────────────────────────────────────────────────────

/// Confirma la instalación del componente MCP mostrando ruta, config y versión.
pub fn mcp_installed(path: &std::path::Path, config: &std::path::Path, version: &str) {
    let path_val = path.display().to_string().cyan().to_string();
    let cfg_val = config.display().to_string().cyan().to_string();
    print_a(
        &t!("mcp.installed"),
        &[
            ("path", &path_val),
            ("config", &cfg_val),
            ("version", version),
        ],
    );
}

/// Confirma la desinstalación del componente MCP.
pub fn mcp_uninstalled() {
    print_b(&t!("mcp.uninstalled"));
}

/// Informa que el componente MCP no está instalado.
pub fn mcp_not_installed() {
    println!("\n{}\n", t!("mcp.not_installed").dimmed());
}

/// Confirma la instalación del skill mostrando ruta y versión.
pub fn skill_installed(path: &std::path::Path, version: &str) {
    let path_val = path.display().to_string().cyan().to_string();
    print_a(
        &t!("skill.installed"),
        &[("path", &path_val), ("version", version)],
    );
}

/// Confirma la desinstalación del skill.
pub fn skill_uninstalled() {
    print_b(&t!("skill.uninstalled"));
}

/// Informa que el skill no está instalado.
pub fn skill_not_installed() {
    println!("\n{}\n", t!("skill.not_installed").dimmed());
}

// ─── Migrate ──────────────────────────────────────────────────────────────────

/// Muestra las instrucciones de ayuda para el comando `migrate` con las rutas de DB disponibles.
pub fn migrate_help(bottle_name: Option<&str>, local_path: &str, global_path: &str, help: &str) {
    let bottle_val = bottle_name
        .map(|n| n.to_string())
        .unwrap_or_else(|| t!("migrate.help.no_bottle").to_string());
    let rows = vec![
        [t!("migrate.help.bottle").bold().to_string(), bottle_val],
        [
            t!("migrate.help.local").bold().to_string(),
            local_path.to_string(),
        ],
        [
            t!("migrate.help.global").bold().to_string(),
            global_path.to_string(),
        ],
    ];
    println!("\n{}\n\n{}\n", table::dict(rows), help);
}

/// Muestra el resumen de confirmación antes de migrar datos de local a global.
pub fn migrate_confirm_global(
    bottle_name: &str,
    local_path: &str,
    global_path: &str,
    prescriptions: usize,
    pills: usize,
) {
    let rows = vec![
        [
            t!("migrate.global.bottle").bold().to_string(),
            bottle_name.to_string(),
        ],
        [
            t!("migrate.global.origin").bold().to_string(),
            local_path.to_string(),
        ],
        [
            t!("migrate.global.dest").bold().to_string(),
            global_path.to_string(),
        ],
        [
            t!("migrate.global.prescriptions").bold().to_string(),
            prescriptions.to_string(),
        ],
        [
            t!("migrate.global.pills").bold().to_string(),
            pills.to_string(),
        ],
    ];
    println!("{}\n", table::dict(rows));
    println!("  {}\n", t!("migrate.global.warning").dimmed());
}

/// Muestra el resumen de confirmación antes de migrar datos de global a local.
pub fn migrate_confirm_local(
    bottle_name: &str,
    local_path: &str,
    global_path: &str,
    prescriptions: usize,
    pills: usize,
    will_create: bool,
) {
    let dest_val = if will_create {
        format!("{}  {}", local_path, t!("migrate.local.dest_new").dimmed())
    } else {
        local_path.to_string()
    };
    let rows = vec![
        [
            t!("migrate.local.bottle").bold().to_string(),
            bottle_name.to_string(),
        ],
        [
            t!("migrate.local.origin").bold().to_string(),
            global_path.to_string(),
        ],
        [t!("migrate.local.dest").bold().to_string(), dest_val],
        [
            t!("migrate.local.prescriptions").bold().to_string(),
            prescriptions.to_string(),
        ],
        [
            t!("migrate.local.pills").bold().to_string(),
            pills.to_string(),
        ],
    ];
    println!("{}\n", table::dict(rows));
    println!("  {}\n", t!("migrate.local.warning").dimmed());
}

/// Confirma el resultado de una migración local→global con el recuento de registros movidos.
pub fn migrate_result_global(prescriptions: usize, pills: usize) {
    let p = prescriptions.to_string();
    let pi = pills.to_string();
    let done = t!("migrate.result.done").to_string();
    let deleted = t!("migrate.result.removed_local").to_string();
    print_a(
        &done,
        &[("prescriptions", &p), ("pills", &pi), ("deleted", &deleted)],
    );
}

/// Confirma el resultado de una migración global→local con el recuento de registros movidos.
pub fn migrate_result_local(prescriptions: usize, pills: usize) {
    let p = prescriptions.to_string();
    let pi = pills.to_string();
    let done = t!("migrate.result.done").to_string();
    let removed = t!("migrate.result.removed_global").to_string();
    print_a(
        &done,
        &[("prescriptions", &p), ("pills", &pi), ("removed", &removed)],
    );
}

// ─── Prescription show ────────────────────────────────────────────────────────

/// Muestra el detalle completo de una prescripción con sus pills activas y archivadas.
///
/// Las pills activas se limitan a `limit`. Las archivadas se limitan a
/// `archived_limit`; cuando `archived_total > archived_limit` se imprime
/// el trailer `... N más archivados` con `N = archived_total - archived_limit`.
/// Si `archived_limit == 0`, oculta la sección de archivadas entera.
pub fn prescription_show(
    rx: &Prescription,
    pills: &[pillbox::domain::pill::Pill],
    limit: u32,
    archived_limit: u32,
    archived_total: u32,
) {
    let estado = if rx.ended_at.is_some() {
        t!("prescriptions.state.closed").dimmed().to_string()
    } else {
        t!("prescriptions.state.open").green().to_string()
    };
    let short_id = &rx.id[..rx.id.len().min(8)];
    let author_val = rx.author_name.as_deref().unwrap_or("-").to_string();
    let rows = vec![
        [
            t!("prescription.show.id").bold().to_string(),
            short_id.cyan().to_string(),
        ],
        [
            t!("prescription.show.title").bold().to_string(),
            rx.title.clone(),
        ],
        [t!("prescription.show.state").bold().to_string(), estado],
        [
            t!("prescription.show.started").bold().to_string(),
            rx.started_at.clone(),
        ],
        [
            t!("prescription.show.author").bold().to_string(),
            author_val,
        ],
    ];
    println!("\n{}", table::dict(rows));

    let (active_pills, archived_pills): (
        Vec<&pillbox::domain::pill::Pill>,
        Vec<&pillbox::domain::pill::Pill>,
    ) = pills.iter().partition(|p| p.deleted_at.is_none());

    let total_active = active_pills.len();
    let shown_active: Vec<&pillbox::domain::pill::Pill> = active_pills.iter().copied().take(limit as usize).collect();
    let hidden = total_active.saturating_sub(shown_active.len());

    let archived_cap = archived_limit as usize;
    let archived_shown: Vec<&pillbox::domain::pill::Pill> = if archived_cap == 0 {
        Vec::new()
    } else {
        archived_pills.iter().copied().take(archived_cap).collect()
    };

    println!(
        "\nPills  {}  {}    {}",
        "·".dimmed(),
        short_id.dimmed(),
        total_active.to_string().bold()
    );

    if active_pills.is_empty() && archived_shown.is_empty() {
        println!("\n  {}\n", t!("pills.none").dimmed());
        return;
    }

    if !shown_active.is_empty() {
        println!();
        let table_rows = shown_active
            .iter()
            .map(|p| {
                vec![
                    p.id.to_string(),
                    truncate(&p.compound, 16),
                    truncate(&p.title, 50),
                    p.author_name.as_deref().map(|n| truncate(n, 18)).unwrap_or_else(|| "-".dimmed().to_string()),
                ]
            })
            .collect();
        print!(
            "{}",
            table::plain_list(
                &[
                    t!("pills.list.col.num").as_ref(),
                    t!("pills.list.col.compound").as_ref(),
                    t!("pills.list.col.title").as_ref(),
                    t!("pills.list.col.author").as_ref(),
                ],
                table_rows,
            )
        );
        if hidden > 0 {
            println!("   … {} más", hidden);
        } else {
            println!();
        }
    }

    if !archived_shown.is_empty() {
        let n = archived_shown.len();
        println!(
            "\n  {}\n",
            t!("prescription.pills.archived_section", count = n)
                .dimmed()
                .to_string()
        );
        let table_rows = archived_shown
            .iter()
            .map(|p| {
                vec![
                    p.id.to_string().dimmed().to_string(),
                    truncate(&p.compound, 16).dimmed().to_string(),
                    truncate(&p.title, 50).dimmed().to_string(),
                    p.author_name.as_deref().map(|n| truncate(n, 18).dimmed().to_string()).unwrap_or_else(|| "-".dimmed().to_string()),
                ]
            })
            .collect();
        print!(
            "{}",
            table::plain_list(
                &[
                    t!("pills.list.col.num").as_ref(),
                    t!("pills.list.col.compound").as_ref(),
                    t!("pills.list.col.title").as_ref(),
                    t!("pills.list.col.author").as_ref(),
                ],
                table_rows,
            )
        );
        let archived_hidden = archived_total.saturating_sub(archived_cap as u32);
        if archived_hidden > 0 {
            println!(
                "   {}",
                t!("prescription.pills.more_archived", count = archived_hidden).dimmed()
            );
        }
        println!();
    } else {
        println!();
    }
}

// ─── Pill detail ──────────────────────────────────────────────────────────────

/// Muestra el detalle completo de una pill: metadatos y contenido formateado.
pub fn pill_detail(pill: &Pill) {
    let short_id = format!("id: {}", pill.id);
    let rx_short = &pill.prescription_id[..pill.prescription_id.len().min(8)];
    let author_val = pill.author_name.as_deref().unwrap_or("-").to_string();
    let rows = vec![
        [t!("pill.detail.id").bold().to_string(), short_id],
        [
            t!("pill.detail.compound").bold().to_string(),
            pill.compound.clone(),
        ],
        [
            t!("pill.detail.title").bold().to_string(),
            pill.title.clone(),
        ],
        [
            t!("pill.detail.prescription").bold().to_string(),
            rx_short.cyan().to_string(),
        ],
        [
            t!("pill.detail.created").bold().to_string(),
            pill.created_at.clone(),
        ],
        [
            t!("pill.detail.author").bold().to_string(),
            author_val,
        ],
    ];
    println!("\n{}", table::dict(rows));
    println!();
    for line in pill.content.lines() {
        println!("  {}", line);
    }
    println!();
}

// ─── Capsules list ────────────────────────────────────────────────────────────

/// Muestra la lista de capsules, separando activas de archivadas.
///
/// Caller pre-divide los registros entre `active` y `archived`. La función
/// aplica el cap visible (`archived_limit`) sobre los archivados y muestra
/// el trailer `... N más archivados` con `N = archived_total - archived_limit`
/// cuando el total exceda el cap. Si `archived_limit == 0`, oculta toda la
/// sección de archivadas.
pub fn capsules_list(
    active: &[Capsule],
    archived: &[Capsule],
    archived_limit: u32,
    archived_total: u32,
    total: u32,
) {
    let count = active.len();
    println!("\nCapsules    {}", count.to_string().bold());

    let archived_cap = archived_limit as usize;
    let archived_shown: Vec<&Capsule> = if archived_cap == 0 {
        Vec::new()
    } else {
        archived.iter().take(archived_cap).collect()
    };

    if active.is_empty() && archived_shown.is_empty() {
        println!("\n  {}\n", t!("capsules.none").dimmed());
        return;
    }

    if !active.is_empty() {
        println!();
        let rows = active
            .iter()
            .map(|c| {
                vec![
                    c.id.to_string(),
                    truncate(&c.compound, 14),
                    truncate(&c.title, 50),
                ]
            })
            .collect();
        print!(
            "{}",
            table::plain_list(
                &[
                    t!("capsules.list.col.num").as_ref(),
                    t!("capsules.list.col.compound").as_ref(),
                    t!("capsules.list.col.title").as_ref(),
                ],
                rows,
            )
        );
        let active_count = active.len() as u32;
        let hidden = total.saturating_sub(active_count + archived_total);
        if hidden > 0 {
            println!("   … {} más", hidden);
        } else {
            println!();
        }
    }

    if !archived_shown.is_empty() {
        let n = archived_shown.len();
        println!(
            "\n  {}\n",
            t!("capsules.list.archived_section", count = n)
                .dimmed()
                .to_string()
        );
        let rows = archived_shown
            .iter()
            .map(|c| {
                let archived_date = c
                    .deleted_at
                    .as_deref()
                    .and_then(|d| d.get(..10))
                    .unwrap_or("—")
                    .dimmed()
                    .to_string();
                vec![
                    c.id.to_string().dimmed().to_string(),
                    truncate(&c.compound, 14).dimmed().to_string(),
                    truncate(&c.title, 46).dimmed().to_string(),
                    archived_date,
                ]
            })
            .collect();
        print!(
            "{}",
            table::plain_list(
                &[
                    t!("capsules.list.col.num").as_ref(),
                    t!("capsules.list.col.compound").as_ref(),
                    t!("capsules.list.col.title").as_ref(),
                    t!("capsules.list.col.archived_at").as_ref(),
                ],
                rows,
            )
        );
        let archived_hidden = archived_total.saturating_sub(archived_cap as u32);
        if archived_hidden > 0 {
            println!(
                "   {}",
                t!("capsules.list.more_archived", count = archived_hidden).dimmed()
            );
        }
        println!();
    } else {
        println!();
    }
}

// ─── Capsule detail ───────────────────────────────────────────────────────────

/// Muestra el detalle completo de una capsule: metadatos y contenido formateado.
pub fn capsule_detail(capsule: &Capsule) {
    let short_id = format!("id: {}", capsule.id);
    let updated = if capsule.updated_at != capsule.created_at {
        capsule.updated_at.dimmed().to_string()
    } else {
        "—".dimmed().to_string()
    };
    let rows = vec![
        [t!("capsule.detail.id").bold().to_string(), short_id],
        [
            t!("capsule.detail.compound").bold().to_string(),
            capsule.compound.clone(),
        ],
        [
            t!("capsule.detail.title").bold().to_string(),
            capsule.title.clone(),
        ],
        [
            t!("capsule.detail.created").bold().to_string(),
            capsule.created_at.clone(),
        ],
        [t!("capsule.detail.updated").bold().to_string(), updated],
    ];
    println!("\n{}", table::dict(rows));
    println!();
    for line in capsule.content.lines() {
        println!("  {}", line);
    }
    println!();
}
