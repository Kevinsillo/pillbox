use super::{table, truncate};
use owo_colors::OwoColorize;
use pillbox::domain::{bottle::Bottle, prescription::Prescription};
use rust_i18n::t;

// ─── Bottles ──────────────────────────────────────────────────────────────────

pub fn bottles_list(bottles: &[Bottle]) {
    if bottles.is_empty() {
        println!("{}\n", t!("bottles.none"));
        return;
    }
    println!("{}\n", t!("bottles.list.title", count = bottles.len()).bold());
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
        table::list(
            &[
                t!("bottles.list.col.num").as_ref(),
                t!("bottles.list.col.name").as_ref(),
                t!("bottles.list.col.display").as_ref(),
                t!("bottles.list.col.dir").as_ref(),
            ],
            rows
        )
    );
}

pub fn bottle_status(bottle: &Bottle, pill_count: i64, open_rx: Option<(String, String)>) {
    let rx_val = match open_rx {
        Some((id, title)) => format!(
            "{}",
            t!("bottle.status.rx_open", title = title, id = &id[..id.len().min(8)])
        ),
        None => t!("bottle.status.rx_none").dimmed().to_string(),
    };
    let rows = vec![
        [
            t!("bottle.status.labels.bottle").bold().to_string(),
            format!("{} — \"{}\"", bottle.name, bottle.display_name),
        ],
        [t!("bottle.status.labels.scope").bold().to_string(), bottle.scope.to_string()],
        [t!("bottle.status.labels.dir").bold().to_string(), bottle.directory.clone()],
        [t!("bottle.status.labels.pills").bold().to_string(), pill_count.to_string()],
        [t!("bottle.status.labels.rx").bold().to_string(), rx_val],
    ];
    println!("{}\n", table::dict(rows));
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
    println!("{}\n", t!("status.title").bold());

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
                val.push_str(&format!("\n{}  \"{}\"", t!("status.db.rx").bold(), title.green()));
            }
        }
        val
    };

    let server_val = match server_port {
        Some(port) => format!("{} {}", "●".green(), t!("status.server.running", port = port)),
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
        [t!("status.labels.bin").bold().to_string(), bin_path.to_string()],
        [t!("status.labels.global").bold().to_string(), db_val(global, None, false)],
        [t!("status.labels.local").bold().to_string(), db_val(local, bottle, true)],
        [t!("status.labels.web").bold().to_string(), server_val],
        [t!("status.labels.mcp").bold().to_string(), mcp_val],
        [t!("status.labels.skill").bold().to_string(), skill_val],
    ];
    println!("{}\n", table::dict(rows));
}

pub fn serve_status(running: bool, pid: Option<u32>, port: u16) {
    println!("{}\n", t!("serve.title").bold());
    let estado = if running {
        format!("{} {}", "●".green(), t!("serve.running"))
    } else {
        format!("{} {}", "●".red(), t!("serve.stopped"))
    };
    let mut rows = vec![
        [t!("serve.labels.estado").bold().to_string(), estado],
        [t!("serve.labels.url").bold().to_string(), format!("http://localhost:{}", port)],
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
    let content = format!("{}\n\n{}: {}{}", title, t!("status.component.estado").bold(), status, rest);
    let rows = vec![["".to_string(), content]];
    println!("{}\n", table::dict(rows));
}

pub fn db_not_found() {
    println!("{}\n", t!("db.not_found"));
}

// ─── Pills ────────────────────────────────────────────────────────────────────

pub fn pills_list(bottle_name: &str, pills: &[(i64, String, String, String)]) {
    println!("{}\n", t!("pills.list.title", bottle = bottle_name).bold());
    let rows = if pills.is_empty() {
        vec![vec!["".into(), t!("pills.none").dimmed().to_string(), "".into()]]
    } else {
        pills
            .iter()
            .enumerate()
            .map(|(i, (_, compound, title, _))| {
                vec![
                    (i + 1).to_string(),
                    truncate(compound, 16),
                    truncate(title, 50),
                ]
            })
            .collect()
    };
    println!(
        "{}\n",
        table::list(
            &[
                t!("pills.list.col.num").as_ref(),
                t!("pills.list.col.compound").as_ref(),
                t!("pills.list.col.title").as_ref(),
            ],
            rows
        )
    );
}

// ─── Prescriptions ────────────────────────────────────────────────────────────

pub fn prescriptions_list(bottle_name: &str, rxs: &[Prescription], limit: u32) {
    println!(
        "{}\n",
        t!("prescriptions.list.title", bottle = bottle_name, limit = limit).bold()
    );
    if rxs.is_empty() {
        println!("  {}\n", t!("prescriptions.none").dimmed());
        return;
    }
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
        table::list(
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
    println!("{} {}", "●".green().bold(), t!("prescriptions.msg.opened", title = title));
    println!("  {}{}\n", t!("prescriptions.msg.opened_id"), id.dimmed());
}

pub fn prescription_closed(title: &str) {
    println!("{} {}\n", "●".green().bold(), t!("prescriptions.msg.closed", title = title));
}

// ─── Bottle init ─────────────────────────────────────────────────────────────

pub fn bottle_init_start(dir: &str) {
    println!("{}\n", t!("bottle.init.start", dir = dir));
}

pub fn bottle_init_created(name: &str, display_name: &str, db_path: &std::path::Path) {
    println!("{} {}\n", "●".green().bold(), t!("bottle.init.created", name = name));
    let rows = vec![
        [t!("bottle.init.col.slug").bold().to_string(), name.to_string()],
        [t!("bottle.init.col.display").bold().to_string(), display_name.to_string()],
        [t!("bottle.init.col.db").bold().to_string(), db_path.display().to_string()],
    ];
    println!("{}\n", table::dict(rows));
}

pub fn bottle_init_gitignore() {
    println!("{} {}", "●".green().bold(), t!("bottle.init.gitignore.done"));
}

pub fn bottle_init_done() {
    println!("{}\n", t!("bottle.init.done"));
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
    println!("{}\n", t!("migrate.title", name = bottle_name, direction = direction).bold());
    let mut rows = vec![
        [t!("migrate.col.bottles").bold().to_string(), bottles.to_string()],
        [t!("migrate.col.prescriptions").bold().to_string(), prescriptions.to_string()],
        [t!("migrate.col.pills").bold().to_string(), pills.to_string()],
    ];
    if let Some(c) = capsules {
        rows.push([t!("migrate.col.capsules").bold().to_string(), c.to_string()]);
    }
    println!("{}\n", table::dict(rows));
}
