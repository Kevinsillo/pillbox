use anyhow::Result;
use owo_colors::OwoColorize;
use rust_i18n::t;

use crate::skill_assets;

pub fn cmd_skill_install() -> Result<()> {
    let skill_path = pillbox::config::skill_path();
    let skill_dir = skill_path.parent().unwrap().to_path_buf();
    std::fs::create_dir_all(&skill_dir)?;

    for file in skill_assets::SkillAssets::iter() {
        let content = skill_assets::SkillAssets::get(&file).unwrap();
        let dest = skill_dir.join(file.as_ref());
        if let Some(parent) = dest.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&dest, content.data.as_ref())?;
    }

    println!(
        "{} {}\n",
        "●".green().bold(),
        t!("skill.installed", path = skill_path.display())
    );
    Ok(())
}

pub fn cmd_skill_uninstall() -> Result<()> {
    let skill_dir = pillbox::config::skill_path()
        .parent()
        .unwrap()
        .to_path_buf();
    if !skill_dir.exists() {
        println!("{}\n", t!("skill.not_installed"));
        return Ok(());
    }
    std::fs::remove_dir_all(&skill_dir)?;
    println!("{} {}\n", "●".green().bold(), t!("skill.uninstalled"));
    Ok(())
}
