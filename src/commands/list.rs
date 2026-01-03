use anyhow::{bail, Result};
use crate::config::ConfigManager;
use crate::db::ConfigDatabase;
use crate::encryption::FileEncryptor;
use crate::sync::FileSyncer;
use crate::utils::{print_error, print_section};
use colored::Colorize;
use std::collections::HashMap;

pub fn execute(all: bool, stub_filters: Vec<String>) -> Result<()> {
    let repo_path = ConfigManager::resolve_repo_path()?;
    let manager = ConfigManager::new(repo_path.clone());

    if !manager.is_initialized() {
        print_error("Not in a dotfiles repository. Run 'dotfiles init' first.");
        bail!("Repository not initialized");
    }

    if all {
        list_all_available(&repo_path)?;
    } else {
        show_status(&manager, &repo_path, stub_filters)?;
    }

    Ok(())
}

fn show_status(manager: &ConfigManager, repo_path: &std::path::PathBuf, stub_filters: Vec<String>) -> Result<()> {
    let tracked = manager.load_tracked_files()?;

    if tracked.is_empty() {
        println!("No files are tracked yet.");
        println!("\nUse {} to start tracking files.", "dotfiles add <stub>".cyan());
        return Ok(());
    }

    print_section("File Status");

    // Get encryption key if needed
    let has_encrypted = tracked.iter().any(|f| f.encrypted);
    let encryption_key = if has_encrypted {
        get_encryption_key_if_needed(repo_path).ok()
    } else {
        None
    };

    let mut by_stub: HashMap<String, Vec<(String, FileStatus)>> = HashMap::new();

    for file in tracked {
        let stub_name = file.stub.clone().unwrap_or_else(|| "direct".to_string());
        
        // Apply stub filter if provided
        if !stub_filters.is_empty() && !stub_filters.contains(&stub_name) {
            continue;
        }
        
        let status = check_file_status(repo_path, &file.path, file.encrypted, encryption_key.as_ref());
        
        // Only add files that are tracked (not "not managed")
        if status != FileStatus::NotManaged {
            by_stub.entry(stub_name).or_default().push((file.path, status));
        }
    }

    if by_stub.is_empty() {
        if !stub_filters.is_empty() {
            println!("No tracked files found for the specified stubs.");
        } else {
            println!("No files are tracked yet.");
        }
        return Ok(());
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
                    FileStatus::MissingInRepo => "?".blue(),
                    FileStatus::NotManaged => "−".dimmed(),
                };
                let status_text = match status {
                    FileStatus::InSync => "in sync",
                    FileStatus::OutOfSync => "out of sync",
                    FileStatus::MissingInRepo => "missing in repo",
                    FileStatus::NotManaged => "not managed",
                };
                println!("  {} {} {}", status_str, path, format!("({})", status_text).dimmed());
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

#[derive(Debug, PartialEq)]
enum FileStatus {
    InSync,
    OutOfSync,
    MissingInRepo,
    NotManaged,
}

fn check_file_status(repo_path: &std::path::PathBuf, home_path: &str, encrypted: bool, encryption_key: Option<&[u8; 32]>) -> FileStatus {
    let home_full = FileSyncer::expand_tilde(home_path);
    let repo_file = repo_path.join(home_path.trim_start_matches("~/").trim_start_matches('/'));

    let home_exists = home_full.exists();
    
    // For encrypted files, check the .enc version in repo
    let repo_exists = if encrypted {
        let encrypted_path = repo_file.with_extension("enc");
        encrypted_path.exists()
    } else {
        repo_file.exists()
    };

    match (home_exists, repo_exists) {
        // File doesn't exist locally - mark as not managed (don't show in list)
        (false, _) => FileStatus::NotManaged,
        // File exists locally but not in repo
        (true, false) => FileStatus::MissingInRepo,
        // File exists in both locations
        (true, true) => {
            if encrypted {
                // Compare encrypted file with home file
                if let Some(key) = encryption_key {
                    if files_are_same_encrypted(&home_full, &repo_file.with_extension("enc"), key) {
                        FileStatus::InSync
                    } else {
                        FileStatus::OutOfSync
                    }
                } else {
                    // Can't decrypt, assume out of sync
                    FileStatus::OutOfSync
                }
            } else {
                // Compare unencrypted files
                if files_are_same(&home_full, &repo_file) {
                    FileStatus::InSync
                } else {
                    FileStatus::OutOfSync
                }
            }
        }
    }
}

fn get_encryption_key_if_needed(repo_path: &std::path::PathBuf) -> Result<[u8; 32]> {
    if FileEncryptor::has_local_key() {
        FileEncryptor::load_key_from_home()
    } else if FileEncryptor::is_encryption_setup(repo_path) {
        // Ask for seed phrase
        let mnemonic = FileEncryptor::prompt_for_seed_phrase()?;
        let key = FileEncryptor::derive_key_from_mnemonic(&mnemonic);
        FileEncryptor::save_key_to_home(&key)?;
        Ok(key)
    } else {
        bail!("No encryption key found")
    }
}

fn files_are_same(path1: &std::path::Path, path2: &std::path::Path) -> bool {
    use std::io::Read;
    
    if path1.is_dir() != path2.is_dir() {
        return false;
    }

    if path1.is_dir() {
        return true;
    }

    // Compare file contents directly
    match (std::fs::File::open(path1), std::fs::File::open(path2)) {
        (Ok(mut f1), Ok(mut f2)) => {
            let mut buf1 = Vec::new();
            let mut buf2 = Vec::new();
            
            if f1.read_to_end(&mut buf1).is_err() || f2.read_to_end(&mut buf2).is_err() {
                return false;
            }
            
            buf1 == buf2
        }
        _ => false,
    }
}

fn files_are_same_encrypted(plaintext_path: &std::path::Path, encrypted_path: &std::path::Path, key: &[u8; 32]) -> bool {
    use std::io::Read;
    
    // Decrypt the encrypted file to temp and compare
    let temp_decrypted = std::env::temp_dir().join(format!("dotfiles_temp_{}", uuid::Uuid::new_v4()));
    
    if FileEncryptor::decrypt_file(encrypted_path, &temp_decrypted, key).is_err() {
        return false;
    }
    
    let result = match (std::fs::File::open(plaintext_path), std::fs::File::open(&temp_decrypted)) {
        (Ok(mut f1), Ok(mut f2)) => {
            let mut buf1 = Vec::new();
            let mut buf2 = Vec::new();
            
            if f1.read_to_end(&mut buf1).is_err() || f2.read_to_end(&mut buf2).is_err() {
                false
            } else {
                buf1 == buf2
            }
        }
        _ => false,
    };
    
    // Clean up temp file
    let _ = std::fs::remove_file(temp_decrypted);
    
    result
}
