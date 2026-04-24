use anyhow::{Context, Result};
use rust_i18n::t;

use crate::output;

use super::shared::{find_current_bottle, open_resolved_db, spinner};

pub fn cmd_bottle_list() -> Result<()> {
    use pillbox::db::{
        connection,
        store::{bottles, registered_bottles},
    };

    let global_path = pillbox::config::global_db_path();
    if !global_path.exists() {
        output::fmt::db_not_found();
        return Ok(());
    }

    let global_conn = connection::open(&global_path)?;
    let registered = registered_bottles::list(&global_conn)?;

    let current = std::env::current_dir().ok();
    let mut rows: Vec<output::fmt::BottleListRow> = Vec::new();

    for reg in registered {
        let db_path = std::path::Path::new(&reg.db_path);
        if !db_path.exists() {
            let directory = db_path
                .parent()
                .and_then(|p| p.parent())
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_else(|| reg.db_path.clone());
            rows.push(output::fmt::BottleListRow {
                name: reg.display_name,
                directory,
                scope: "local".to_string(),
                linked: false,
                is_active: false,
            });
            continue;
        }

        let db_conn = match connection::open(db_path) {
            Ok(c) => c,
            Err(_) => continue,
        };
        for bottle in bottles::list(&db_conn).unwrap_or_default() {
            let is_active = current
                .as_ref()
                .map(|c| c.starts_with(&bottle.directory))
                .unwrap_or(false);
            rows.push(output::fmt::BottleListRow {
                name: bottle.display_name,
                directory: bottle.directory,
                scope: bottle.scope,
                linked: true,
                is_active,
            });
        }
    }

    rows.sort_by(|a, b| a.name.cmp(&b.name));
    output::fmt::bottles_registered_list(&rows);
    Ok(())
}

pub fn cmd_bottle_status() -> Result<()> {
    let (conn, _) = open_resolved_db()?;
    let bottle = find_current_bottle()?;

    let pill_count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM pills p
         JOIN prescriptions rx ON rx.id = p.prescription_id
         WHERE rx.bottle_id = ?1 AND p.deleted_at IS NULL",
        rusqlite::params![bottle.id],
        |r| r.get(0),
    )?;

    let open_rx: Option<(String, String)> = conn
        .query_row(
            "SELECT id, title FROM prescriptions
             WHERE bottle_id = ?1 AND ended_at IS NULL AND deleted_at IS NULL
             LIMIT 1",
            rusqlite::params![bottle.id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .ok();

    output::fmt::bottle_status(&bottle, pill_count, open_rx);
    Ok(())
}

pub fn cmd_bottle_init() -> Result<()> {
    use inquire::{Confirm, Select, Text};
    use pillbox::db::{connection, store::bottles};
    use pillbox::domain::bottle::{BottleScope, NewBottle};

    let current_dir = std::env::current_dir()?;
    let dir_str = current_dir.to_string_lossy().to_string();
    let dir_basename = current_dir
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();

    output::fmt::bottle_init_start(&dir_str);

    let display_name = Text::new(&t!("bottle.init.name_prompt"))
        .with_default(&dir_basename)
        .prompt()?;

    let name = pillbox::normalize::bottle_name(&display_name);

    let scope_options = vec![
        t!("bottle.init.scope.local").to_string(),
        t!("bottle.init.scope.global").to_string(),
    ];
    let scope_choice = Select::new(&t!("bottle.init.scope.prompt"), scope_options).prompt()?;
    let scope = if scope_choice.starts_with("local") {
        BottleScope::Local
    } else {
        BottleScope::Global
    };

    let db_path = match &scope {
        BottleScope::Local => pillbox::config::local_db_path(),
        BottleScope::Global => pillbox::config::global_db_path(),
    };

    let pb = spinner(t!("bottle.init.creating"));
    let mut conn = connection::open(&db_path)?;

    if bottles::find_by_directory(&conn, &dir_str)?.is_some() {
        pb.finish_and_clear();
        anyhow::bail!("{}", t!("bottle.init.exists", path = db_path.display()));
    }

    let bottle = bottles::create(
        &mut conn,
        &NewBottle {
            name: name.clone(),
            display_name: display_name.clone(),
            directory: dir_str.clone(),
            scope: scope.clone(),
        },
    )?;
    pb.finish_and_clear();

    output::fmt::bottle_init_created(&bottle.name, &bottle.display_name, &db_path);

    if scope == BottleScope::Local && current_dir.join(".git").exists() {
        let add_gi = Confirm::new(&t!("bottle.init.gitignore.prompt"))
            .with_default(false)
            .prompt()
            .unwrap_or(false);
        if add_gi {
            add_to_gitignore(&current_dir)?;
            output::fmt::bottle_init_gitignore();
        }
    }

    {
        let global_path = pillbox::config::global_db_path();
        let _ = connection::open(&global_path);
        let pb2 = spinner(t!("bottle.init.registering"));
        // Para scope local: registrar la DB local. Para scope global: registrar la DB global.
        let register_db_path: std::path::PathBuf = match &scope {
            BottleScope::Local => current_dir.join(".pillbox").join("pillbox.db"),
            BottleScope::Global => global_path.clone(),
        };
        match register_in_global(
            &global_path,
            &bottle.id,
            &name,
            &display_name,
            &register_db_path,
        ) {
            Ok(_) => pb2.finish_with_message(t!("bottle.init.registered_ok").to_string()),
            Err(e) => {
                pb2.finish_and_clear();
                eprintln!("{}", t!("bottle.init.register_err", err = e));
            }
        }
    }

    output::fmt::bottle_init_done();
    Ok(())
}

pub fn cmd_bottle_delete(slug: &str) -> Result<()> {
    use inquire::Text;
    use owo_colors::OwoColorize;
    use pillbox::db::{connection, store::registered_bottles};

    let global_path = pillbox::config::global_db_path();
    if !global_path.exists() {
        output::fmt::db_not_found();
        return Ok(());
    }

    let global_conn = connection::open(&global_path)?;
    let reg = registered_bottles::list(&global_conn)?
        .into_iter()
        .find(|r| r.name == slug);

    let Some(reg) = reg else {
        eprintln!(
            "\n{}  {}\n",
            "✗".red().bold(),
            t!("bottle.delete.not_found", slug = slug)
        );
        return Ok(());
    };

    let name_label = t!("bottle.delete.col.name");
    let slug_label = t!("bottle.delete.col.slug");
    let path_label = t!("bottle.delete.col.path");
    let key_width = [&name_label, &slug_label, &path_label]
        .iter()
        .map(|k| k.len())
        .max()
        .unwrap_or(0)
        + 1;

    println!("\n{}  {}", "⚑".yellow().bold(), t!("bottle.delete.header"));
    println!(
        "   {:<w$}│  {}",
        name_label.dimmed(),
        reg.display_name,
        w = key_width
    );
    println!(
        "   {:<w$}│  {}",
        slug_label.dimmed(),
        reg.name,
        w = key_width
    );
    println!(
        "   {:<w$}│  {}",
        path_label.dimmed(),
        reg.db_path.dimmed(),
        w = key_width
    );
    println!();

    let input = Text::new(&t!("bottle.delete.confirm", slug = reg.name).to_string()).prompt()?;

    if input.trim() != reg.name {
        eprintln!(
            "\n{}  {}\n",
            "✗".red().bold(),
            t!("bottle.delete.wrong_slug")
        );
        return Ok(());
    }

    registered_bottles::unregister(&global_conn, reg.id)?;

    println!("\n{}  {}", "✓".green().bold(), t!("bottle.delete.done"));
    println!(
        "   {:<w$}│  {}\n",
        slug_label.dimmed(),
        reg.name,
        w = key_width
    );
    Ok(())
}

pub fn cmd_bottle_repair(slug: &str) -> Result<()> {
    use inquire::Text;
    use owo_colors::OwoColorize;
    use pillbox::db::{connection, store::registered_bottles};

    let global_path = pillbox::config::global_db_path();
    if !global_path.exists() {
        output::fmt::db_not_found();
        return Ok(());
    }

    let global_conn = connection::open(&global_path)?;
    let reg = registered_bottles::list(&global_conn)?
        .into_iter()
        .find(|r| r.name == slug);

    let Some(reg) = reg else {
        eprintln!(
            "\n{}  {}\n",
            "✗".red().bold(),
            t!("bottle.repair.not_found", slug = slug)
        );
        return Ok(());
    };

    if std::path::Path::new(&reg.db_path).exists() {
        eprintln!(
            "\n{}  {}\n",
            "✗".red().bold(),
            t!("bottle.repair.already_linked", slug = slug)
        );
        return Ok(());
    }

    println!(
        "\n  {} {}",
        t!("bottle.repair.header"),
        reg.display_name.bold()
    );
    println!(
        "  {}    {}",
        t!("bottle.repair.current_path"),
        reg.db_path.dimmed()
    );
    println!();

    let new_path = Text::new(&t!("bottle.repair.prompt").to_string())
        .with_help_message(&t!("bottle.repair.prompt_help").to_string())
        .prompt()?;

    let new_path = new_path.trim().to_string();
    let path = std::path::Path::new(&new_path);

    if !path.exists() || !path.is_file() {
        eprintln!(
            "\n{}  {}\n",
            "✗".red().bold(),
            t!("bottle.repair.invalid_path")
        );
        return Ok(());
    }

    registered_bottles::update_db_path(&global_conn, reg.id, &new_path)?;

    let slug_label = t!("bottle.repair.col.slug");
    let path_label = t!("bottle.repair.col.path");
    let key_width = [&slug_label, &path_label]
        .iter()
        .map(|k| k.len())
        .max()
        .unwrap_or(0)
        + 1;

    println!("\n{}  {}", "✓".green().bold(), t!("bottle.repair.done"));
    println!(
        "   {:<w$}│  {}",
        slug_label.dimmed(),
        reg.name,
        w = key_width
    );
    println!(
        "   {:<w$}│  {}\n",
        path_label.dimmed(),
        new_path,
        w = key_width
    );
    Ok(())
}

fn add_to_gitignore(dir: &std::path::Path) -> Result<()> {
    use std::io::Write;
    let gi_path = dir.join(".gitignore");
    let entry = ".pillbox/\n";

    if gi_path.exists() {
        let content = std::fs::read_to_string(&gi_path)?;
        if content
            .lines()
            .any(|l| l.trim() == ".pillbox/" || l.trim() == ".pillbox")
        {
            return Ok(());
        }
        let mut file = std::fs::OpenOptions::new().append(true).open(&gi_path)?;
        if !content.ends_with('\n') {
            file.write_all(b"\n")?;
        }
        file.write_all(entry.as_bytes())?;
    } else {
        std::fs::write(&gi_path, entry)?;
    }
    Ok(())
}

fn register_in_global(
    global_path: &std::path::Path,
    bottle_id: &str,
    name: &str,
    display_name: &str,
    local_db_path: &std::path::Path,
) -> Result<()> {
    use pillbox::db::{connection, store::registered_bottles};

    let conn = connection::open(global_path)?;
    let db_path_str = local_db_path
        .to_str()
        .context("local DB path contains non-UTF-8 characters")?;
    registered_bottles::register(&conn, bottle_id, name, display_name, db_path_str)?;

    Ok(())
}

pub fn cmd_migrate_help() -> Result<()> {
    use pillbox::db::{connection, store::bottles};

    let global_path = pillbox::config::global_db_path();
    let local_path = pillbox::config::local_db_path();

    let bottle_name: Option<String> = if global_path.exists() {
        let conn = connection::open(&global_path).ok();
        conn.and_then(|c| {
            let dir = std::env::current_dir().ok()?.to_string_lossy().to_string();
            bottles::find_by_directory(&c, &dir)
                .ok()
                .flatten()
                .map(|b| b.name)
        })
    } else {
        None
    };

    output::fmt::migrate_help(
        bottle_name.as_deref(),
        &local_path.display().to_string(),
        &global_path.display().to_string(),
    );
    Ok(())
}

pub fn cmd_migrate_global() -> Result<()> {
    use inquire::Confirm;
    use pillbox::db::{connection, migrate};

    let global_path = pillbox::config::global_db_path();
    let local_path = pillbox::config::local_db_path();

    if !global_path.exists() {
        anyhow::bail!(
            "{}",
            t!("migrate.error.no_global", path = global_path.display())
        );
    }
    if !local_path.exists() {
        anyhow::bail!("{}", t!("migrate.error.no_local"));
    }

    let src_conn = connection::open(&local_path)?;
    let current_dir = std::env::current_dir()?;
    let dir_str = current_dir.to_string_lossy();

    let bottle_name: String = src_conn
        .query_row(
            "SELECT name FROM bottles WHERE directory = ?1",
            rusqlite::params![dir_str.as_ref()],
            |r| r.get(0),
        )
        .map_err(|_| {
            anyhow::anyhow!(
                "{}",
                t!(
                    "migrate.error.no_bottle",
                    dir = dir_str,
                    path = local_path.display()
                )
            )
        })?;

    let (prescriptions, pills) = migrate::count_bottle_contents(&src_conn, &bottle_name)?;

    output::fmt::migrate_confirm_global(
        &bottle_name,
        &local_path.display().to_string(),
        &global_path.display().to_string(),
        prescriptions,
        pills,
    );

    let confirmed = Confirm::new(&t!("migrate.confirm.prompt"))
        .with_default(false)
        .prompt()?;

    if !confirmed {
        return Ok(());
    }

    let mut dst_conn = connection::open(&global_path)?;
    let result = migrate::migrate_bottle(&src_conn, &mut dst_conn, &bottle_name)?;
    drop(src_conn);

    std::fs::remove_file(&local_path)
        .with_context(|| "failed to remove local DB after migration")?;

    output::fmt::migrate_result_global(result.prescriptions, result.pills);
    Ok(())
}

pub fn cmd_migrate_local() -> Result<()> {
    use inquire::{Confirm, Select};
    use pillbox::db::{connection, migrate, store::bottles};

    let global_path = pillbox::config::global_db_path();
    let local_path = pillbox::config::local_db_path();

    if !global_path.exists() {
        anyhow::bail!(
            "{}",
            t!("migrate.error.no_global", path = global_path.display())
        );
    }

    if local_path.exists() {
        let local_conn = connection::open(&local_path).ok();
        let has_bottle = local_conn.and_then(|c| {
            let dir = std::env::current_dir().ok()?.to_string_lossy().to_string();
            bottles::find_by_directory(&c, &dir).ok().flatten()
        });
        if has_bottle.is_some() {
            anyhow::bail!("{}", t!("migrate.local.error.already_local"));
        }
    }

    let global_conn = connection::open(&global_path)?;
    let bottle_list = migrate::list_bottles_with_counts(&global_conn)?;

    if bottle_list.is_empty() {
        anyhow::bail!("{}", t!("migrate.local.error.no_bottles"));
    }

    let options: Vec<String> = bottle_list
        .iter()
        .map(|(name, dir, pill_count)| format!("{}  —  {}  ({} pills)", name, dir, pill_count))
        .collect();

    let selection = Select::new(&t!("migrate.local.select"), options.clone()).prompt()?;

    let idx = options.iter().position(|o| o == &selection).unwrap_or(0);
    let (bottle_name, _bottle_dir, _) = &bottle_list[idx];
    let bottle_name = bottle_name.clone();

    let (prescriptions, pills) = migrate::count_bottle_contents(&global_conn, &bottle_name)?;

    let will_create = !local_path.exists();

    output::fmt::migrate_confirm_local(
        &bottle_name,
        &local_path.display().to_string(),
        &global_path.display().to_string(),
        prescriptions,
        pills,
        will_create,
    );

    let confirmed = Confirm::new(&t!("migrate.confirm.prompt"))
        .with_default(false)
        .prompt()?;

    if !confirmed {
        return Ok(());
    }

    let mut dst_conn = connection::open(&local_path)?;
    let result = migrate::migrate_bottle(&global_conn, &mut dst_conn, &bottle_name)?;
    drop(global_conn);

    let mut global_conn_mut = connection::open(&global_path)?;
    migrate::delete_bottle(&mut global_conn_mut, &bottle_name)?;

    output::fmt::migrate_result_local(result.prescriptions, result.pills);
    Ok(())
}
