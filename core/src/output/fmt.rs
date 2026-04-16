use super::{table, truncate};
use owo_colors::OwoColorize;
use pillbox::domain::{bottle::Bottle, prescription::Prescription};

// ─── Bottles ──────────────────────────────────────────────────────────────────

pub fn bottles_list(bottles: &[Bottle]) {
    if bottles.is_empty() {
        println!("No hay bottles registrados.\n");
        return;
    }
    println!(
        "{}\n",
        format!("Bottles registrados ({})", bottles.len()).bold()
    );
    let rows = bottles
        .iter()
        .enumerate()
        .map(|(i, b)| {
            vec![
                (i + 1).to_string(),
                truncate(&b.name, 22),
                truncate(&b.display_name, 22),
                b.directory.clone(),
            ]
        })
        .collect();
    println!(
        "{}\n",
        table::list(&["#", "Nombre", "Display", "Directorio"], rows)
    );
}

pub fn bottle_status(bottle: &Bottle, pill_count: i64, open_rx: Option<(String, String)>) {
    let rx_val = match open_rx {
        Some((id, title)) => format!(
            "\"{}\" {}",
            title,
            format!("(abierta, id={})", &id[..id.len().min(8)]).dimmed()
        ),
        None => "ninguna abierta".dimmed().to_string(),
    };
    let rows = vec![
        [
            format!("{}", "Bottle".bold()),
            format!("{} — \"{}\"", bottle.name, bottle.display_name),
        ],
        [format!("{}", "Scope".bold()), bottle.scope.to_string()],
        [format!("{}", "Directorio".bold()), bottle.directory.clone()],
        [format!("{}", "Pills".bold()), pill_count.to_string()],
        [format!("{}", "Rx".bold()), rx_val],
    ];
    println!("{}\n", table::dict(rows));
}

// ─── Status ───────────────────────────────────────────────────────────────────

pub struct StatusDb {
    pub path: String,
    pub result: Option<Result<(i64, i64, i64, i64), String>>,
}

pub struct StatusBottle {
    pub name: String,
    pub display_name: String,
    pub open_rx: Option<String>,
}

pub fn status(
    bin_path: &str,
    global: StatusDb,
    local: StatusDb,
    bottle: Option<StatusBottle>,
    server_port: Option<u16>,
    mcp_path: &std::path::Path,
    skill_path: &std::path::Path,
) {
    println!("{}\n", "Pillbox Status".bold());

    let db_val = |s: StatusDb, bottle: Option<StatusBottle>| -> String {
        let mut val = match s.result {
            None => return format!("{}\n{}", s.path, "(no existe)".dimmed()),
            Some(Ok((v, btl, pills, caps))) => format!(
                "{}\n{}  {}{}  {}{}  {}{}  {}{}",
                s.path,
                "✓".green(),
                "Schema: ".bold(),
                format!("v{}", v).green(),
                "Bottles: ".bold(),
                btl.to_string().green(),
                "Pills: ".bold(),
                pills.to_string().green(),
                "Capsules: ".bold(),
                caps.to_string().green(),
            ),
            Some(Err(e)) => format!("{}\n{} {}", s.path, "✗".red(), e),
        };
        if let Some(b) = bottle {
            val.push_str(&format!(
                "\n{}  {} — \"{}\"",
                "Bottle: ".bold(),
                b.name,
                b.display_name
            ));
            if let Some(title) = b.open_rx {
                val.push_str(&format!("\n{}  \"{}\"", "Rx: ".bold(), title.green()));
            }
        }
        val
    };

    let server_val = match server_port {
        Some(port) => format!("{} http://localhost:{}", "✓".green(), port),
        None => format!("{} no está en ejecución", "✗".red()),
    };

    let mcp_val = if mcp_path.exists() {
        format!("{} {}", "✓".green(), mcp_path.display())
    } else {
        format!("{} no instalado", "✗".red())
    };

    let skill_val = if skill_path.exists() {
        format!("{} {}", "✓".green(), skill_path.display())
    } else {
        format!("{} no instalada", "✗".red())
    };

    let rows = vec![
        [format!("{}", "Binario".bold()), bin_path.to_string()],
        [format!("{}", "Global Bottle".bold()), db_val(global, None)],
        [format!("{}", "Local Bottle".bold()), db_val(local, bottle)],
        [format!("{}", "Servidor Web".bold()), server_val],
        [format!("{}", "MCP".bold()), mcp_val],
        [format!("{}", "Skill".bold()), skill_val],
    ];
    println!("{}\n", table::dict(rows));
}

pub fn db_not_found() {
    println!("No se encontró ninguna Pillbox. Ejecuta el script de instalación.\n");
}

// ─── Pills ────────────────────────────────────────────────────────────────────

pub fn pills_list(bottle_name: &str, pills: &[(i64, String, String, String)], limit: u32) {
    println!(
        "{}\n",
        format!("Pills de '{}' (últimas {})", bottle_name, limit).bold()
    );
    if pills.is_empty() {
        println!("  {}\n", "(ninguna)".dimmed());
        return;
    }
    let rows = pills
        .iter()
        .enumerate()
        .map(|(i, (_, compound, title, _))| {
            vec![
                (i + 1).to_string(),
                truncate(compound, 16),
                truncate(title, 50),
            ]
        })
        .collect();
    println!("{}\n", table::list(&["#", "Compound", "Título"], rows));
}

// ─── Prescriptions ────────────────────────────────────────────────────────────

pub fn prescriptions_list(bottle_name: &str, rxs: &[Prescription], limit: u32) {
    println!(
        "{}\n",
        format!("Prescriptions de '{}' (últimas {})", bottle_name, limit).bold()
    );
    if rxs.is_empty() {
        println!("  {}\n", "(ninguna)".dimmed());
        return;
    }
    let rows = rxs
        .iter()
        .map(|rx| {
            let short_id = &rx.id[..rx.id.len().min(8)];
            let estado = if rx.ended_at.is_some() {
                "cerrada".dimmed().to_string()
            } else {
                "abierta".green().to_string()
            };
            vec![short_id.to_string(), truncate(&rx.title, 40), estado]
        })
        .collect();
    println!("{}\n", table::list(&["ID", "Título", "Estado"], rows));
}

pub fn prescription_opened(id: &str, title: &str) {
    println!("{} Prescripción abierta: \"{}\"", "✓".green().bold(), title);
    println!("  ID: {}\n", id.dimmed());
}

pub fn prescription_closed(title: &str) {
    println!(
        "{} Prescripción cerrada: \"{}\"\n",
        "✓".green().bold(),
        title
    );
}

// ─── Bottle init ─────────────────────────────────────────────────────────────

pub fn bottle_init_start(dir: &str) {
    println!("Inicializando bottle en {}...\n", dir);
}

pub fn bottle_init_created(name: &str, display_name: &str, db_path: &std::path::Path) {
    println!("{} Bottle '{}' creado.\n", "✓".green().bold(), name);
    let rows = vec![
        ["Slug".bold().to_string(), name.to_string()],
        ["Display".bold().to_string(), display_name.to_string()],
        ["DB".bold().to_string(), db_path.display().to_string()],
    ];
    println!("{}\n", table::dict(rows));
}

pub fn bottle_init_gitignore() {
    println!("{} .gitignore actualizado.", "✓".green().bold());
}

pub fn bottle_init_done() {
    println!("Listo. Usa 'pillbox bottle status' para ver el estado.\n");
}

// ─── Migrate ──────────────────────────────────────────────────────────────────

pub fn migrate_result(
    direction: &str,
    bottle_name: &str,
    bottles: usize,
    prescriptions: usize,
    pills: usize,
    capsules: Option<usize>,
) {
    println!(
        "Migrando bottle '{}' ({})...\n",
        bottle_name.bold(),
        direction
    );
    let mut rows = vec![
        [format!("{}", "Bottles".bold()), bottles.to_string()],
        [
            format!("{}", "Prescripciones".bold()),
            prescriptions.to_string(),
        ],
        [format!("{}", "Pills".bold()), pills.to_string()],
    ];
    if let Some(c) = capsules {
        rows.push([format!("{}", "Capsules".bold()), c.to_string()]);
    }
    println!("{}\n", table::dict(rows));
}
