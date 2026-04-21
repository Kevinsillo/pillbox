use anyhow::Result;
use owo_colors::OwoColorize;
use rust_i18n::t;

use crate::output;

use super::install;

pub fn cmd_mcp_install() -> Result<()> {
    let node_ok = std::process::Command::new("node")
        .arg("--version")
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .and_then(|v| {
            v.trim()
                .trim_start_matches('v')
                .split('.')
                .next()?
                .parse::<u32>()
                .ok()
        })
        .map(|maj| maj >= 18)
        .unwrap_or(false);

    if !node_ok {
        anyhow::bail!("{}", t!("mcp.node_required"));
    }

    let mcp_dir = pillbox::config::mcp_path().parent().unwrap().to_path_buf();
    let claude_cfg = pillbox::config::claude_config_path();

    println!("\n  {}", t!("mcp.downloading").dimmed());
    let version = install::mcp::install(&mcp_dir, &claude_cfg)?;

    output::fmt::mcp_installed(&mcp_dir, &claude_cfg, &version);
    Ok(())
}

pub fn cmd_mcp_uninstall() -> Result<()> {
    let mcp_dir = pillbox::config::mcp_path().parent().unwrap().to_path_buf();
    let claude_cfg = pillbox::config::claude_config_path();

    let removed = install::mcp::uninstall(&mcp_dir, &claude_cfg)?;
    if removed {
        output::fmt::mcp_uninstalled();
    } else {
        output::fmt::mcp_not_installed();
    }
    Ok(())
}
