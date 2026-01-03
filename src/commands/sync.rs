use anyhow::{bail, Context, Result};
use colored::Colorize;
use crate::config::{ConfigManager, TrackedFile};
use crate::encryption::FileEncryptor;
use crate::git::GitRepo;
use crate::sync::FileSyncer;
use crate::utils::{print_error, print_info, print_success, print_warning};

pub fn execute(sync_all: bool, sync_encrypted: bool, dir: Option<std::path::PathBuf>, _password: Option<String>) -> Result<()> {
    // Handle --dir argument to change and save repo directory
    let repo_path = if let Some(dir_path) = dir {
        let canonical_path = dir_path.canonicalize()
            .context("Failed to resolve directory path")?;
        
        // Save to local config
        let temp_manager = ConfigManager::new(canonical_path.clone());
        temp_manager.save_local_config(canonical_path.clone())?;
        print_success(&format!("Saved dotfiles directory to local config: {}", canonical_path.display()));
        
        canonical_path
    } else {
        std::env::current_dir()?
    };
    
    let manager = ConfigManager::new(repo_path.clone());
    let git = GitRepo::new(&repo_path);

    // --- PRE-FLIGHT CHECKS ---
    if !manager.is_initialized() {
        print_error("Not in a dotfiles repository. Run 'dotfiles init' first.");
        bail!("Repository not initialized");
    }

    if !git.is_repo() {
        print_error("Not a git repository. Initialize git first.");
        bail!("Not a git repository");
    }

    // --- SETUP ---
    let tracked = manager.load_tracked_files()?;
    let files_to_sync = filter_files(&tracked, sync_all, sync_encrypted);
    
    if files_to_sync.is_empty() {
        print_info("No files to sync with current filter");
        return Ok(());
    }

    let encryption_key = resolve_encryption_key(&repo_path, &tracked, sync_all, sync_encrypted)?;

    // Check for remote and warn if local-only
    let has_remote = git.has_remote()?;
    if !has_remote {
        print_warning("⚠️  No remote repository configured - backup is LOCAL ONLY");
        println!("   Add a remote with: git remote add origin <url>");
        println!();
    }

    print_info("Starting robust bidirectional sync...");
    println!();

    // --- STEP 1: IMPORT (Home -> Repo) ---
    print_info("Step 1/5: Importing local changes...");
    sync_home_to_repo(&manager, &files_to_sync, encryption_key.as_ref())?;

    // --- STEP 2: STAGE & COMMIT ---
    // Check if the import actually changed anything in the repo structure
    if git.is_dirty()? {
        print_info("Step 2/5: Committing local changes...");
        git.add_all()?;
        
        let timestamp = chrono::Local::now().format("%Y-%m-%d %H:%M:%S");
        git.commit(&format!("dotfiles sync: {}", timestamp))?;
        print_success("Local changes committed");
    } else {
        print_info("No local changes to commit.");
    }

    // --- STEP 3: SYNC REMOTE (Pull --rebase) ---
    let branch = git.get_current_branch()?;
    let remote_is_empty = if has_remote {
        // Check if remote has commits before attempting pull
        !git.remote_has_commits("origin", &branch)?
    } else {
        false
    };
    
    if has_remote && !remote_is_empty {
        print_info("Step 3/6: Pulling updates from remote...");
        
        // We use fetch + rebase for a cleaner history and safety.
        // If rebase fails, it returns error, and we DO NOT proceed to Step 4.
        match git.pull_rebase("origin", &branch) {
            Ok(_) => print_success("Remote updates applied"),
            Err(e) => {
                print_error("Merge conflict during update!");
                println!("\n{}", "SAFETY LOCK ENGAGED: Home directory was NOT updated.".yellow().bold());
                println!("  1. Go to repository directory");
                println!("  2. Resolve conflicts manually");
                println!("  3. Run 'git rebase --continue'");
                println!("  4. Run 'dotfiles sync' again");
                // Stop execution to protect the Home directory from conflict markers
                return Err(e); 
            }
        }
    } else if has_remote && remote_is_empty {
        print_info("Remote is empty - skipping pull (first push)");
    } else {
        print_info("No remote configured - skipping pull");
    }

    // --- STEP 4: BACKUP & EXPORT (Repo -> Home) ---
    // We only reach here if Step 3 succeeded (Repo is clean, merged, and valid)
    print_info("Step 4/6: Creating backup of current home files...");
    let backup_created = backup_home_files(&repo_path, &files_to_sync, encryption_key.as_ref())?;
    if backup_created {
        print_success("Backup created");
    } else {
        print_info("No files to backup (first sync or files don't exist)");
    }
    
    print_info("Step 5/6: Exporting to Home directory...");
    sync_repo_to_home(&manager, &files_to_sync, encryption_key.as_ref())?;

    // Commit backup if it was created
    if backup_created && git.is_dirty()? {
        git.add_all()?;
        let timestamp = chrono::Local::now().format("%Y-%m-%d %H:%M:%S");
        git.commit(&format!("backup: pre-export snapshot {}", timestamp))?;
        print_success("Backup committed to repository");
    }

    // --- STEP 6: PUSH ---
    if has_remote {
        print_info("Step 6/6: Pushing to remote (including backups)...");
        
        // Use push with upstream tracking for first push to empty remote
        if remote_is_empty {
            git.push_set_upstream("origin", &branch)?;
            print_success("Pushed successfully (set upstream tracking)");
        } else {
            git.push("origin", &branch)?;
            print_success("Pushed successfully");
        }
    }

    println!();
    print_success("Sync completed successfully!");
    Ok(())
}

// --- HELPER FUNCTIONS ---

fn filter_files(tracked: &[TrackedFile], sync_all: bool, sync_encrypted: bool) -> Vec<TrackedFile> {
    if sync_encrypted {
        tracked.iter().filter(|f| f.encrypted).cloned().collect()
    } else if sync_all {
        tracked.to_vec()
    } else {
        tracked.iter().filter(|f| !f.encrypted).cloned().collect()
    }
}

fn resolve_encryption_key(
    repo_path: &std::path::Path,
    tracked: &[TrackedFile], 
    sync_all: bool, 
    sync_encrypted: bool
) -> Result<Option<[u8; 32]>> {
    let needs_encryption = sync_encrypted || (sync_all && tracked.iter().any(|f| f.encrypted));
    
    if needs_encryption {
        if FileEncryptor::is_encryption_setup(repo_path) {
            // Load existing key from repo
            Ok(Some(FileEncryptor::load_key_from_repo(repo_path)?))
        } else {
            // Key not found - check if there are encrypted files in the repo
            // This happens when cloning a repo with encrypted files to a new machine
            let has_encrypted_files_in_repo = check_for_encrypted_files_in_repo(repo_path);
            
            if has_encrypted_files_in_repo {
                print_info("Encrypted files detected but encryption key not found.");
                print_info("Please enter your 12-word seed phrase to restore encryption.");
                
                let mnemonic = FileEncryptor::prompt_for_seed_phrase()?;
                let key = FileEncryptor::derive_key_from_mnemonic(&mnemonic);
                FileEncryptor::save_key_to_repo(repo_path, &key)?;
                print_success("Encryption key restored and saved to repository");
                
                Ok(Some(key))
            } else {
                bail!("No encrypted files found. Use 'dotfiles add --encrypt <file>' to add encrypted files.");
            }
        }
    } else {
        Ok(None)
    }
}

fn check_for_encrypted_files_in_repo(repo_path: &std::path::Path) -> bool {
    use walkdir::WalkDir;
    
    for entry in WalkDir::new(repo_path).max_depth(5) {
        if let Ok(entry) = entry {
            if entry.path().extension().and_then(|s| s.to_str()) == Some("enc") {
                return true;
            }
        }
    }
    false
}

fn backup_home_files(repo_path: &std::path::Path, files: &[TrackedFile], encryption_key: Option<&[u8; 32]>) -> Result<bool> {
    use std::fs;
    
    // Create timestamp directory name
    let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S");
    let backup_dir = repo_path.join(".backup").join(timestamp.to_string());
    
    let mut any_backed_up = false;
    
    for file in files {
        let home_path = FileSyncer::expand_tilde(&file.path);
        
        // Only backup if file exists in home
        if !home_path.exists() {
            continue;
        }
        
        // Create backup path mirroring the home structure
        let relative_path = file.path.trim_start_matches("~/").trim_start_matches('/');
        let backup_file = backup_dir.join(relative_path);
        
        // Create parent directory
        if let Some(parent) = backup_file.parent() {
            fs::create_dir_all(parent)?;
        }
        
        // Copy the file (encrypt if needed for encrypted files)
        if file.encrypted {
            // For encrypted files, backup them encrypted with .enc extension
            if let Some(key) = encryption_key {
                let encrypted_backup = backup_file.with_extension("enc");
                FileEncryptor::encrypt_file(&home_path, &encrypted_backup, key)?;
                any_backed_up = true;
            }
        } else {
            FileSyncer::sync_file(&home_path, &backup_file)?;
            any_backed_up = true;
        }
    }
    
    // If no files were backed up, remove the empty directory
    if !any_backed_up && backup_dir.exists() {
        fs::remove_dir_all(&backup_dir).ok();
    }
    
    Ok(any_backed_up)
}

fn sync_home_to_repo(manager: &ConfigManager, files: &[TrackedFile], encryption_key: Option<&[u8; 32]>) -> Result<()> {
    let repo_path = manager.get_repo_path();

    for file in files {
        let home_path = FileSyncer::expand_tilde(&file.path);
        
        if !home_path.exists() {
            continue;
        }
        
        let repo_file = repo_path.join(file.path.trim_start_matches("~/").trim_start_matches('/'));

        if file.encrypted {
            if let Some(key) = encryption_key {
                let encrypted_path = repo_file.with_extension("enc");
                FileEncryptor::encrypt_file(&home_path, &encrypted_path, key)?;
            }
        } else {
            FileSyncer::sync_file(&home_path, &repo_file)?;
        }
    }
    Ok(())
}

fn sync_repo_to_home(manager: &ConfigManager, files: &[TrackedFile], encryption_key: Option<&[u8; 32]>) -> Result<()> {
    let repo_path = manager.get_repo_path();

    for file in files {
        let home_path = FileSyncer::expand_tilde(&file.path);
        let repo_file = repo_path.join(file.path.trim_start_matches("~/").trim_start_matches('/'));

        if file.encrypted {
            if let Some(key) = encryption_key {
                let encrypted_path = repo_file.with_extension("enc");
                if encrypted_path.exists() {
                    // Create parent directory if it doesn't exist
                    if let Some(parent) = home_path.parent() {
                        std::fs::create_dir_all(parent)?;
                    }
                    FileEncryptor::decrypt_file(&encrypted_path, &home_path, key)?;
                }
            }
        } else if repo_file.exists() {
            // Create parent directory if it doesn't exist
            if let Some(parent) = home_path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            FileSyncer::sync_file(&repo_file, &home_path)?;
        }
    }
    Ok(())
}
