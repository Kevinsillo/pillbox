use anyhow::Result;
use owo_colors::OwoColorize;
use rust_i18n::t;

use crate::mcp_assets;

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
    std::fs::create_dir_all(&mcp_dir)?;

    for file in mcp_assets::McpAssets::iter() {
        let content = mcp_assets::McpAssets::get(&file).unwrap();
        let dest = mcp_dir.join(file.as_ref());
        if let Some(parent) = dest.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&dest, content.data.as_ref())?;
    }

    println!(
        "{} {}\n",
        "●".green().bold(),
        t!("mcp.installed", path = mcp_dir.display())
    );

    let claude_cfg = pillbox::config::claude_config_path();
    let index_js = mcp_dir.join("index.js");
    let entry = serde_json::json!({
        "command": "node",
        "args": [index_js.to_string_lossy()]
    });

    let mut root: serde_json::Value = if claude_cfg.exists() {
        let raw = std::fs::read_to_string(&claude_cfg)?;
        serde_json::from_str(&raw).unwrap_or(serde_json::json!({}))
    } else {
        serde_json::json!({})
    };

    root["mcpServers"]["pillbox"] = entry;
    std::fs::write(&claude_cfg, serde_json::to_string_pretty(&root)? + "\n")?;

    println!(
        "{} {}\n",
        "●".green().bold(),
        t!("mcp.claude_json_updated", path = claude_cfg.display())
    );
    Ok(())
}

pub fn cmd_mcp_uninstall() -> Result<()> {
    let mcp_dir = pillbox::config::mcp_path().parent().unwrap().to_path_buf();
    if !mcp_dir.exists() {
        println!("{}\n", t!("mcp.not_installed"));
        return Ok(());
    }
    std::fs::remove_dir_all(&mcp_dir)?;
    println!("{} {}\n", "●".green().bold(), t!("mcp.uninstalled"));
    Ok(())
}
