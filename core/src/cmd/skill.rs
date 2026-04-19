use anyhow::Result;
use owo_colors::OwoColorize;
use rust_i18n::t;

use super::install;

pub fn cmd_skill_install() -> Result<()> {
    let skill_path = pillbox::config::skill_path();
    let skill_dir = skill_path.parent().unwrap().to_path_buf();

    println!("{}", t!("skill.downloading"));
    let version = install::skill::install(&skill_dir)?;

    println!(
        "{} {}\n",
        "●".green().bold(),
        t!("skill.installed", path = skill_path.display())
    );
    println!("{} {}\n", "●".green().bold(), version);
    Ok(())
}

pub fn cmd_skill_uninstall() -> Result<()> {
    let skill_dir = pillbox::config::skill_path()
        .parent()
        .unwrap()
        .to_path_buf();

    let removed = install::skill::uninstall(&skill_dir)?;
    if removed {
        println!("{} {}\n", "●".green().bold(), t!("skill.uninstalled"));
    } else {
        println!("{}\n", t!("skill.not_installed"));
    }
    Ok(())
}
