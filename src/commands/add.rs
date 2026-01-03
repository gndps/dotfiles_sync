use anyhow::{bail, Context, Result};
use crate::config::{ConfigManager, TrackedFile};
use crate::db::ConfigDatabase;
use crate::encryption::FileEncryptor;
use crate::sync::FileSyncer;
use crate::utils::{print_success, print_error, print_info};

pub fn execute(stubs_or_paths: Vec<String>, encrypt: bool, _password: Option<String>) -> Result<()> {
    let repo_path = std::env::current_dir()?;
    let manager = ConfigManager::new(repo_path.clone());

    if !manager.is_initialized() {
        print_error("Not in a dotfiles repository. Run 'dotfiles init' first.");
        bail!("Repository not initialized");
    }

    if stubs_or_paths.is_empty() {
        print_error("No stubs or paths provided.");
        bail!("No stubs or paths provided");
    }

    let mut tracked = manager.load_tracked_files()?;
    
    // Handle encryption setup if needed
    let encryption_key = if encrypt {
        Some(setup_encryption_if_needed(&repo_path)?)
    } else {
        None
    };
    
    // Process each stub or path
    for stub_or_path in stubs_or_paths {
        // Check if it's a direct path or a stub
        let is_direct_path = stub_or_path.contains('/') || stub_or_path.starts_with('~') || stub_or_path.starts_with('.');
        
        if is_direct_path {
            // Direct file/folder path
            if let Err(e) = add_direct_path(&repo_path, &manager, &mut tracked, &stub_or_path, encrypt, encryption_key.as_ref()) {
                print_error(&format!("Failed to add {}: {}", stub_or_path, e));
            }
        } else {
            // Stub from database
            if let Err(e) = add_from_stub(&repo_path, &manager, &mut tracked, &stub_or_path, encrypt, encryption_key.as_ref()) {
                print_error(&format!("Failed to add {}: {}", stub_or_path, e));
            }
        }
    }
    
    manager.save_tracked_files(&tracked)?;
    Ok(())
}

fn setup_encryption_if_needed(repo_path: &std::path::Path) -> Result<[u8; 32]> {
    if FileEncryptor::is_encryption_setup(repo_path) {
        // Encryption already set up, load the key
        print_info("Using existing encryption key from repository");
        FileEncryptor::load_key_from_repo(repo_path)
    } else {
        // First time encryption - generate seed phrase
        print_info("Setting up encryption for the first time...");
        
        let mnemonic = FileEncryptor::generate_mnemonic()?;
        FileEncryptor::display_seed_phrase(&mnemonic);
        
        // Prompt user to confirm they saved it
        use std::io::{self, Write};
        print!("Type 'yes' to confirm you have saved the seed phrase: ");
        io::stdout().flush()?;
        
        let mut confirmation = String::new();
        io::stdin().read_line(&mut confirmation)?;
        
        if confirmation.trim().to_lowercase() != "yes" {
            bail!("Encryption setup cancelled. Please save your seed phrase before continuing.");
        }
        
        // Derive and save the key
        let key = FileEncryptor::derive_key_from_mnemonic(&mnemonic);
        FileEncryptor::save_key_to_repo(repo_path, &key)?;
        print_success("Encryption key saved to repository");
        
        Ok(key)
    }
}

fn add_from_stub(
    repo_path: &std::path::Path,
    _manager: &ConfigManager,
    tracked: &mut Vec<TrackedFile>,
    stub: &str,
    encrypt: bool,
    encryption_key: Option<&[u8; 32]>
) -> Result<()> {
    let db = ConfigDatabase::new(repo_path);
    let entry = db.load_stub(stub)?;
    
    if entry.is_none() {
        print_error(&format!("Stub '{}' not found in database", stub));
        print_info("Use 'dotfiles list --all' to see available stubs");
        print_info("Or use 'dotfiles create <stub> <paths...>' to create a custom stub");
        bail!("Stub not found");
    }

    let entry = entry.unwrap();
    let files_to_track = entry.config_files.clone();

    if files_to_track.is_empty() {
        print_error(&format!("No files configured for stub '{}'", stub));
        bail!("No files to track");
    }

    print_info(&format!("Adding {} ({})...", entry.name, stub));

    for file_path in &files_to_track {
        let (home_path, full_home_path) = resolve_file_path(file_path);
        
        if full_home_path.exists() {
            let repo_file_path = repo_path.join(file_path.trim_start_matches("~/").trim_start_matches('/'));
            
            if let Some(key) = encryption_key {
                let encrypted_path = repo_file_path.with_extension("enc");
                FileEncryptor::encrypt_file(&full_home_path, &encrypted_path, key)
                    .context(format!("Failed to encrypt {}", home_path))?;
                print_success(&format!("Encrypted and copied: {}", home_path));
            } else {
                FileSyncer::sync_file(&full_home_path, &repo_file_path)
                    .context(format!("Failed to sync {}", home_path))?;
                print_success(&format!("Copied: {}", home_path));
            }
        } else {
            print_info(&format!("Not found (skipped): {}", home_path));
        }

        if !tracked.iter().any(|t| t.path == home_path) {
            tracked.push(TrackedFile {
                stub: Some(stub.to_string()),
                path: home_path,
                encrypted: encrypt,
            });
        }
    }
    
    Ok(())
}

fn add_direct_path(
    repo_path: &std::path::Path,
    _manager: &ConfigManager,
    tracked: &mut Vec<TrackedFile>,
    path: &str,
    encrypt: bool,
    encryption_key: Option<&[u8; 32]>
) -> Result<()> {
    let expanded_path = FileSyncer::expand_tilde(path);
    
    if !expanded_path.exists() {
        print_error(&format!("Path does not exist: {}", path));
        bail!("Path not found");
    }
    
    // Normalize path to start with ~/
    let normalized_path = if let Some(home) = dirs::home_dir() {
        if let Ok(rel) = expanded_path.strip_prefix(&home) {
            format!("~/{}", rel.display())
        } else {
            path.to_string()
        }
    } else {
        path.to_string()
    };
    
    print_info(&format!("Adding direct path: {}...", normalized_path));
    
    let repo_file_path = repo_path.join(normalized_path.trim_start_matches("~/").trim_start_matches('/'));
    
    if let Some(key) = encryption_key {
        let encrypted_path = repo_file_path.with_extension("enc");
        FileEncryptor::encrypt_file(&expanded_path, &encrypted_path, key)
            .context(format!("Failed to encrypt {}", normalized_path))?;
        print_success(&format!("Encrypted and copied: {}", normalized_path));
    } else {
        FileSyncer::sync_file(&expanded_path, &repo_file_path)
            .context(format!("Failed to sync {}", normalized_path))?;
        print_success(&format!("Copied: {}", normalized_path));
    }
    
    if !tracked.iter().any(|t| t.path == normalized_path) {
        tracked.push(TrackedFile {
            stub: None,
            path: normalized_path.clone(),
            encrypted: encrypt,
        });
        print_success(&format!("Added to tracked files: {}", normalized_path));
    } else {
        print_info(&format!("Already tracked: {}", normalized_path));
    }
    
    Ok(())
}

fn resolve_file_path(file_path: &str) -> (String, std::path::PathBuf) {
    use std::path::PathBuf;
    
    if file_path.starts_with("~/") {
        let path = file_path.to_string();
        let full_path = FileSyncer::expand_tilde(&path);
        return (path, full_path);
    }
    
    if file_path.starts_with('/') {
        let full_path = PathBuf::from(file_path);
        return (file_path.to_string(), full_path);
    }
    
    let candidates = if file_path.starts_with('.') {
        vec![
            format!("~/{}", file_path),
            format!("~/.config/{}", file_path),
            format!("/{}", file_path),
        ]
    } else {
        vec![
            format!("~/{}", file_path),
            format!("~/.config/{}", file_path),
            format!("/{}", file_path),
        ]
    };
    
    for candidate in &candidates {
        let full_path = if candidate.starts_with('/') {
            PathBuf::from(candidate)
        } else {
            FileSyncer::expand_tilde(candidate)
        };
        
        if full_path.exists() {
            return (candidate.clone(), full_path);
        }
    }
    
    (candidates[0].clone(), FileSyncer::expand_tilde(&candidates[0]))
}
