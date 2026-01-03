use anyhow::Result;
use colored::Colorize;
use std::collections::HashMap;
use crate::config::ConfigManager;
use crate::db::ConfigDatabase;
use crate::sync::FileSyncer;
use crate::utils::{print_section, print_info};

pub fn execute() -> Result<()> {
    let repo_path = ConfigManager::resolve_repo_path()?;
    let manager = ConfigManager::new(repo_path.clone());

    if !manager.is_initialized() {
        print_info("Not in a dotfiles repository. Run 'dotfiles init' first.");
        return Ok(());
    }

    print_section("Scanning System for Dotfiles");
    println!();

    let db = ConfigDatabase::new(&repo_path);
    let tracked = manager.load_tracked_files()?;
    
    // Build a map of stub -> tracked status
    let mut tracked_stubs: HashMap<String, bool> = HashMap::new();
    for file in &tracked {
        if let Some(ref stub) = file.stub {
            tracked_stubs.insert(stub.clone(), true);
        }
    }

    // Get all available stubs from database
    let default_stubs = db.get_default_stubs()?;
    let custom_stubs = db.get_custom_stubs()?;
    
    let mut all_stubs: Vec<(String, Vec<String>)> = Vec::new();
    
    for (stub_name, entry) in default_stubs {
        all_stubs.push((stub_name, entry.config_files));
    }
    
    for (stub_name, entry) in custom_stubs {
        all_stubs.push((stub_name, entry.config_files));
    }
    
    all_stubs.sort_by(|a, b| a.0.cmp(&b.0));

    // Categorize stubs
    let mut synced_stubs = Vec::new();
    let mut out_of_sync_stubs = Vec::new();
    let mut unmanaged_stubs = Vec::new();

    for (stub_name, files) in all_stubs {
        // Check if any files from this stub exist on the system
        let mut files_exist = false;
        for file_path in &files {
            let home_path = FileSyncer::expand_tilde(file_path);
            if home_path.exists() {
                files_exist = true;
                break;
            }
        }

        if !files_exist {
            continue; // Skip stubs with no files on system
        }

        // Determine status
        let is_tracked = tracked_stubs.contains_key(&stub_name);
        
        if is_tracked {
            // Check if files are in sync
            let in_sync = check_stub_sync(&repo_path, &files)?;
            if in_sync {
                synced_stubs.push((stub_name, files));
            } else {
                out_of_sync_stubs.push((stub_name, files));
            }
        } else {
            unmanaged_stubs.push((stub_name, files));
        }
    }

    // Print results
    print_results("✓ Synced", &synced_stubs, "green");
    print_results("⚠ Out of Sync", &out_of_sync_stubs, "yellow");
    print_results("○ Unmanaged", &unmanaged_stubs, "cyan");

    // Summary
    println!();
    println!("{}", "Summary:".bold());
    println!("  {} synced", synced_stubs.len().to_string().green());
    println!("  {} out of sync", out_of_sync_stubs.len().to_string().yellow());
    println!("  {} unmanaged", unmanaged_stubs.len().to_string().cyan());
    
    if !unmanaged_stubs.is_empty() {
        println!();
        println!("Tip: Add unmanaged stubs with: {}", "dotfiles add <stub>".cyan());
    }

    Ok(())
}

fn check_stub_sync(repo_path: &std::path::Path, files: &[String]) -> Result<bool> {
    use std::fs;
    
    for file_path in files {
        let home_path = FileSyncer::expand_tilde(file_path);
        
        if !home_path.exists() {
            continue;
        }
        
        let repo_file = repo_path.join(file_path.trim_start_matches("~/").trim_start_matches('/'));
        
        // Check if repo file exists
        if !repo_file.exists() {
                    return Ok(false); // File in home but not in repo
        }
        
        // Compare file contents
        if home_path.is_file() && repo_file.is_file() {
            let home_contents = fs::read(&home_path).ok();
            let repo_contents = fs::read(&repo_file).ok();
            
            if home_contents != repo_contents {
                return Ok(false);
            }
        }
    }
    
    Ok(true)
}

fn print_results(title: &str, stubs: &[(String, Vec<String>)], color: &str) {
    if stubs.is_empty() {
        return;
    }

    println!();
    println!("{}", title.bold());
    
    for (stub_name, files) in stubs {
        println!("\n{}", stub_name.green().bold());
        
        for file_path in files {
            let home_path = FileSyncer::expand_tilde(file_path);
            if home_path.exists() {
                let status_icon = match color {
                    "green" => "✓".green(),
                    "yellow" => "✗".yellow(),
                    "cyan" => "○".cyan(),
                    _ => "?".white(),
                };
                let status_text = match color {
                    "green" => "in sync",
                    "yellow" => "out of sync",
                    "cyan" => "unmanaged",
                    _ => "unknown",
                };
                println!("  {} {} {}", status_icon, file_path, format!("({})", status_text).dimmed());
            }
        }
    }
}
