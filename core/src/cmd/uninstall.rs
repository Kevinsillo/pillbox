use anyhow::Result;
use owo_colors::OwoColorize;
use rust_i18n::t;

pub fn run() -> Result<()> {
    use inquire::Confirm;

    println!("{}\n", t!("uninstall.title").bold());

    let mcp_dir = pillbox::config::mcp_path().parent().unwrap().to_path_buf();
    if mcp_dir.exists() {
        if Confirm::new(&t!("uninstall.mcp.prompt"))
            .with_default(false)
            .prompt()?
        {
            std::fs::remove_dir_all(&mcp_dir)?;
            println!("{} {}", "●".green().bold(), t!("uninstall.mcp.done"));
        }
    }

    let skill_dir = pillbox::config::skill_path()
        .parent()
        .unwrap()
        .to_path_buf();
    if skill_dir.exists() {
        if Confirm::new(&t!("uninstall.skill.prompt"))
            .with_default(false)
            .prompt()?
        {
            std::fs::remove_dir_all(&skill_dir)?;
            println!("{} {}", "●".green().bold(), t!("uninstall.skill.done"));
        }
    }

    let global_db = pillbox::config::global_db_path();
    if global_db.exists() {
        if Confirm::new(&t!("uninstall.db.prompt"))
            .with_default(false)
            .prompt()?
        {
            std::fs::remove_file(&global_db)?;
            println!("{} {}", "●".green().bold(), t!("uninstall.db.done"));
        }
    }

    let bin_path = std::env::current_exe()?;
    if Confirm::new(&t!("uninstall.bin.prompt", path = bin_path.display()))
        .with_default(false)
        .prompt()?
    {
        println!(
            "{}\n",
            t!("uninstall.bin.manual", path = bin_path.display())
        );
    }

    println!();
    Ok(())
}
