//! Panel de estado del sistema: binario, DBs, servidor, MCP y skill.

use super::table;
use owo_colors::OwoColorize;
use rust_i18n::t;

/// Tupla de métricas leídas de una DB para el panel de estado:
/// (schema_version, bottles, pills, capsules, prescriptions).
pub type StatusDbMetrics = (i64, i64, i64, i64, i64);

/// Resultado de consultar las métricas de una DB: `None` si la DB no existe,
/// `Some(Ok(...))` con la tupla de conteos, o `Some(Err(msg))` si la consulta falla.
pub type StatusDbResult = Option<Result<StatusDbMetrics, String>>;

/// Datos de una DB (global o local) para mostrar en el panel de estado.
pub struct StatusDb {
    pub path: String,
    pub result: StatusDbResult,
}

/// Información del bottle activo para mostrar en el panel de estado.
pub struct StatusBottle {
    pub open_rxs: Vec<String>,
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
            for title in &b.open_rxs {
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

/// Confirma en pantalla que el servicio ha arrancado (Pattern A).
pub fn serve_started(url: &str) {
    super::layout::print_a(
        &t!("serve.start.success"),
        &[(t!("serve.labels.url").as_ref(), &url.cyan().to_string())],
    );
}

/// Confirma en pantalla que el servicio se ha detenido (Pattern B).
pub fn serve_stopped() {
    super::layout::print_b(&t!("serve.stop.success"));
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
