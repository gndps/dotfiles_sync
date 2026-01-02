use anyhow::{bail, Result};
use crate::config::ConfigManager;
use crate::db::ConfigDatabase;
use crate::utils::{print_error, print_section};
use colored::Colorize;
use std::collections::HashMap;

pub fn execute(all: bool) -> Result<()> {
    let repo_path = std::env::current_dir()?;
    let manager = ConfigManager::new(repo_path.clone());

    if !manager.is_initialized() {
        print_error("Not in a dotfiles repository. Run 'dotfiles init' first.");
        bail!("Repository not initialized");
    }

    if all {
        list_all_available(&repo_path)?;
    } else {
        list_tracked(&manager)?;
    }

    Ok(())
}

fn list_tracked(manager: &ConfigManager) -> Result<()> {
    let tracked = manager.load_tracked_files()?;

    if tracked.is_empty() {
        println!("No files are currently tracked.");
        println!("\nUse {} to add files.", "dotfiles add <stub>".cyan());
        return Ok(());
    }

    print_section("Tracked Files");

    let mut by_stub: HashMap<String, Vec<String>> = HashMap::new();
    for file in tracked {
        let stub_name = file.stub.clone().unwrap_or_else(|| "direct".to_string());
        by_stub.entry(stub_name).or_default().push(file.path);
    }

    let mut stubs: Vec<_> = by_stub.keys().collect();
    stubs.sort();

    for stub in stubs {
        println!("\n{}", stub.green().bold());
        if let Some(paths) = by_stub.get(stub) {
            for path in paths {
                println!("  {}", path.dimmed());
            }
        }
    }

    Ok(())
}

fn list_all_available(repo_path: &std::path::PathBuf) -> Result<()> {
    let db = ConfigDatabase::new(repo_path);
    let stubs = db.list_all_stubs()?;

    if stubs.is_empty() {
        println!("No stubs available in database.");
        println!("\nCreate a custom stub with:");
        println!("  {}", "dotfiles create <stub> <paths...>".cyan());
        return Ok(());
    }

    print_section("Available Stubs");

    for stub in stubs {
        if let Ok(Some((name, files, is_custom))) = db.get_stub_info(&stub) {
            let stub_type = if is_custom { "custom".magenta() } else { "default".blue() };
            println!("\n{} ({}) [{}]", name.green().bold(), stub.yellow(), stub_type);
            for file in files.iter().take(3) {
                println!("  {}", file.dimmed());
            }
            if files.len() > 3 {
                println!("  {} (and {} more)", "...".dimmed(), files.len() - 3);
            }
        }
    }

    Ok(())
}
