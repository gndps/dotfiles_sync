use anyhow::{bail, Result};
use crate::config::ConfigManager;
use crate::sync::FileSyncer;
use crate::utils::{print_error, print_section};
use colored::Colorize;
use std::collections::HashMap;

#[derive(Debug, PartialEq)]
enum FileStatus {
    InSync,
    OutOfSync,
    MissingInHome,
    MissingInRepo,
}

pub fn execute() -> Result<()> {
    let repo_path = ConfigManager::resolve_repo_path()?;
    let manager = ConfigManager::new(repo_path.clone());

    if !manager.is_initialized() {
        print_error("Not in a dotfiles repository. Run 'dotfiles init' first.");
        bail!("Repository not initialized");
    }

    let tracked = manager.load_tracked_files()?;

    if tracked.is_empty() {
        println!("No files are tracked yet.");
        println!("\nUse {} to start tracking files.", "dotfiles add <stub>".cyan());
        return Ok(());
    }

    print_section("File Status");

    let mut by_stub: HashMap<String, Vec<(String, FileStatus)>> = HashMap::new();

    for file in tracked {
        let status = check_file_status(&repo_path, &file.path);
        let stub_name = file.stub.clone().unwrap_or_else(|| "direct".to_string());
        by_stub.entry(stub_name).or_default().push((file.path, status));
    }

    let mut stubs: Vec<_> = by_stub.keys().collect();
    stubs.sort();

    for stub in stubs {
        println!("\n{}", stub.green().bold());
        if let Some(files) = by_stub.get(stub) {
            for (path, status) in files {
                let status_str = match status {
                    FileStatus::InSync => "✓".green(),
                    FileStatus::OutOfSync => "✗".yellow(),
                    FileStatus::MissingInHome => "⚠".red(),
                    FileStatus::MissingInRepo => "?".blue(),
                };
                let status_text = match status {
                    FileStatus::InSync => "in sync",
                    FileStatus::OutOfSync => "out of sync",
                    FileStatus::MissingInHome => "missing in home",
                    FileStatus::MissingInRepo => "missing in repo",
                };
                println!("  {} {} {}", status_str, path, format!("({})", status_text).dimmed());
            }
        }
    }

    Ok(())
}

fn check_file_status(repo_path: &std::path::PathBuf, home_path: &str) -> FileStatus {
    let home_full = FileSyncer::expand_tilde(home_path);
    let repo_file = repo_path.join(home_path.trim_start_matches("~/"));

    let home_exists = home_full.exists();
    let repo_exists = repo_file.exists();

    match (home_exists, repo_exists) {
        (false, false) => FileStatus::MissingInHome,
        (false, true) => FileStatus::MissingInHome,
        (true, false) => FileStatus::MissingInRepo,
        (true, true) => {
            if files_are_same(&home_full, &repo_file) {
                FileStatus::InSync
            } else {
                FileStatus::OutOfSync
            }
        }
    }
}

fn files_are_same(path1: &std::path::Path, path2: &std::path::Path) -> bool {
    if path1.is_dir() != path2.is_dir() {
        return false;
    }

    if path1.is_dir() {
        return true;
    }

    match (std::fs::metadata(path1), std::fs::metadata(path2)) {
        (Ok(m1), Ok(m2)) => {
            if m1.len() != m2.len() {
                return false;
            }
            
            if let (Ok(t1), Ok(t2)) = (m1.modified(), m2.modified()) {
                (t1.duration_since(std::time::UNIX_EPOCH).unwrap().as_secs() as i64 
                    - t2.duration_since(std::time::UNIX_EPOCH).unwrap().as_secs() as i64).abs() < 2
            } else {
                false
            }
        }
        _ => false,
    }
}
