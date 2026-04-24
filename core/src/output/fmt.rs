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

pub struct BottleListRow {
    pub name: String,
    pub directory: String,
    pub scope: String,
    pub linked: bool,
    pub is_active: bool,
}

pub fn bottles_registered_list(rows: &[BottleListRow]) {
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

    println!(
        "{}\n",
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
}

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
    let rows = vec![
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
        [
            t!("bottle.status.labels.pills").bold().to_string(),
            pill_count.to_string(),
        ],
        [t!("bottle.status.labels.rx").bold().to_string(), rx_val],
    ];
    println!("\n{}\n", table::dict(rows));
}

// ─── Status ───────────────────────────────────────────────────────────────────

pub struct StatusDb {
    pub path: String,
    pub result: Option<Result<(i64, i64, i64, i64, i64), String>>,
}

pub struct StatusBottle {
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

    let rows = vec![
        [
            t!("status.labels.bin").bold().to_string(),
            bin_path.to_string(),
        ],
        [
            t!("status.labels.global").bold().to_string(),
            db_val(global, None, false),
        ],
        [
            t!("status.labels.local").bold().to_string(),
            db_val(local, bottle, true),
        ],
        [t!("status.labels.web").bold().to_string(), server_val],
        [t!("status.labels.mcp").bold().to_string(), mcp_val],
        [t!("status.labels.skill").bold().to_string(), skill_val],
    ];
    println!("{}\n", table::dict(rows));
}

pub fn serve_status(running: bool, pid: Option<u32>, port: u16) {
    let estado = if running {
        format!("{} {}", "●".green(), t!("serve.running"))
    } else {
        format!("{} {}", "●".red(), t!("serve.stopped"))
    };
    let mut rows = vec![
        [t!("serve.labels.estado").bold().to_string(), estado],
        [
            t!("serve.labels.url").bold().to_string(),
            format!("http://localhost:{}", port),
        ],
    ];
    if let Some(p) = pid {
        rows.push([t!("serve.labels.pid").bold().to_string(), p.to_string()]);
    }
    println!("{}\n", table::dict(rows));
}

pub fn component_status_with_help(path: &std::path::Path, help: &str) {
    let status = if path.exists() {
        format!("{} {}", "●".green(), path.display())
    } else {
        format!("{} {}", "●".red(), t!("status.component.missing"))
    };
    let split = help.find("\n\n").unwrap_or(help.len());
    let (title, rest) = help.split_at(split);
    let content = format!(
        "{}\n\n{}: {}{}",
        title,
        t!("status.component.estado").bold(),
        status,
        rest
    );
    let rows = vec![["".to_string(), content]];
    println!("\n{}\n", table::dict(rows));
}

pub fn db_not_found() {
    eprintln!("\n{} {}\n", "✗".red().bold(), t!("db.not_found"));
}

// ─── Prescriptions ────────────────────────────────────────────────────────────

pub fn prescriptions_list(bottle_name: &str, db_path: &str, rxs: &[Prescription], _limit: u32) {
    let count = rxs.len();
    println!(
        "\nPrescriptions  {}  {}    {}",
        "·".dimmed(),
        bottle_name.dimmed(),
        count.to_string().bold()
    );
    println!("  {}", db_path.dimmed());

    if rxs.is_empty() {
        println!("\n  {}\n", t!("prescriptions.none").dimmed());
        return;
    }

    println!();
    let rows = rxs
        .iter()
        .map(|rx| {
            let short_id = &rx.id[..rx.id.len().min(8)];
            let estado = if rx.ended_at.is_some() {
                t!("prescriptions.state.closed").dimmed().to_string()
            } else {
                t!("prescriptions.state.open").green().to_string()
            };
            vec![short_id.to_string(), truncate(&rx.title, 40), estado]
        })
        .collect();
    println!(
        "{}\n",
        table::plain_list(
            &[
                t!("prescriptions.list.col.id").as_ref(),
                t!("prescriptions.list.col.title").as_ref(),
                t!("prescriptions.list.col.state").as_ref(),
            ],
            rows
        )
    );
}

pub fn prescription_opened(id: &str, title: &str) {
    let short_id = &id[..id.len().min(8)];
    print_a(
        &t!("prescriptions.msg.opened"),
        &[("title", title), ("id", short_id)],
    );
}

pub fn prescription_closed(_title: &str) {
    print_b(&t!("prescriptions.msg.closed"));
}

// ─── Bottle init ──────────────────────────────────────────────────────────────

pub fn bottle_init_start(dir: &str) {
    println!("\n{}\n", t!("bottle.init.start", dir = dir));
}

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

pub fn bottle_init_gitignore() {
    print_b(&t!("bottle.init.gitignore.done"));
}

pub fn bottle_init_done() {
    println!("   {}  {}\n", "→".dimmed(), t!("bottle.init.done").dimmed());
}

// ─── MCP / Skill install ──────────────────────────────────────────────────────

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

pub fn mcp_uninstalled() {
    print_b(&t!("mcp.uninstalled"));
}

pub fn mcp_not_installed() {
    println!("\n{}\n", t!("mcp.not_installed").dimmed());
}

pub fn skill_installed(path: &std::path::Path, version: &str) {
    let path_val = path.display().to_string().cyan().to_string();
    print_a(
        &t!("skill.installed"),
        &[("path", &path_val), ("version", version)],
    );
}

pub fn skill_uninstalled() {
    print_b(&t!("skill.uninstalled"));
}

pub fn skill_not_installed() {
    println!("\n{}\n", t!("skill.not_installed").dimmed());
}

// ─── Migrate ──────────────────────────────────────────────────────────────────

pub fn migrate_help(bottle_name: Option<&str>, local_path: &str, global_path: &str) {
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
        ["".to_string(), "".to_string()],
        [
            "migrate global".green().to_string(),
            t!("migrate.help.cmd_global").to_string(),
        ],
        [
            "migrate local".green().to_string(),
            t!("migrate.help.cmd_local").to_string(),
        ],
    ];
    println!("{}\n", table::dict(rows));
}

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

pub fn prescription_show(rx: &Prescription, pills: &[pillbox::domain::pill::Pill]) {
    let estado = if rx.ended_at.is_some() {
        t!("prescriptions.state.closed").dimmed().to_string()
    } else {
        t!("prescriptions.state.open").green().to_string()
    };
    let short_id = &rx.id[..rx.id.len().min(8)];
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
    ];
    println!("\n{}", table::dict(rows));

    let count = pills.len();
    println!(
        "\nPills  {}  {}    {}",
        "·".dimmed(),
        short_id.dimmed(),
        count.to_string().bold()
    );

    if pills.is_empty() {
        println!("\n  {}\n", t!("pills.none").dimmed());
        return;
    }

    println!();
    let table_rows = pills
        .iter()
        .map(|p| {
            vec![
                p.id.to_string(),
                truncate(&p.compound, 16),
                truncate(&p.title, 50),
            ]
        })
        .collect();
    println!(
        "{}\n",
        table::plain_list(
            &[
                t!("pills.list.col.num").as_ref(),
                t!("pills.list.col.compound").as_ref(),
                t!("pills.list.col.title").as_ref(),
            ],
            table_rows,
        )
    );
}

// ─── Pill detail ──────────────────────────────────────────────────────────────

pub fn pill_detail(pill: &Pill) {
    let short_id = format!("#{}", pill.id);
    let rx_short = &pill.prescription_id[..pill.prescription_id.len().min(8)];
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
    ];
    println!("\n{}", table::dict(rows));
    println!();
    for line in pill.content.lines() {
        println!("  {}", line);
    }
    println!();
}

// ─── Capsule detail ───────────────────────────────────────────────────────────

pub fn capsule_detail(capsule: &Capsule) {
    let short_id = format!("#{}", capsule.id);
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
