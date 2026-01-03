use anyhow::{bail, Context, Result};
use colored::Colorize;
use crate::config::{ConfigManager, TrackedFile};
use crate::encryption::FileEncryptor;
use crate::git::GitRepo;
use crate::sync::FileSyncer;
use crate::utils::{print_error, print_info, print_success, print_warning};
use std::path::{Path, PathBuf};

const TEMP_CONFLICTS_DIR: &str = ".dotfiles_conflicts_temp";

pub fn execute(dir: Option<PathBuf>, encryption_key_path: Option<PathBuf>, _password: Option<String>) -> Result<()> {
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
        let repo_path = ConfigManager::resolve_repo_path()?;
        let manager = ConfigManager::new(repo_path.clone());
        
        // Check if local config exists
        let local_config_path = manager.get_local_config_path();
        
        // If local config doesn't exist, save it automatically
        // This allows running 'dotfiles sync' from the repo to enable global usage
        if !local_config_path.exists() {
            manager.save_local_config(repo_path.clone())?;
            print_success(&format!("Saved dotfiles directory to local config: {}", repo_path.display()));
        }
        
        repo_path
    };
    
    // Handle --encryption-key-path argument
    if let Some(key_path) = encryption_key_path {
        let canonical_key_path = key_path.canonicalize()
            .context("Failed to resolve encryption key path")?;
        
        let temp_manager = ConfigManager::new(repo_path.clone());
        let mut config = temp_manager.load_config()?;
        config.encryption_key_path = Some(canonical_key_path.clone());
        
        // Save to local config
        let local_config_path = temp_manager.get_local_config_path();
        let content = serde_json::to_string_pretty(&config)
            .context("Failed to serialize local config")?;
        std::fs::write(&local_config_path, content)
            .context("Failed to write local config file")?;
        
        print_success(&format!("Saved encryption key path to local config: {}", canonical_key_path.display()));
    }
    
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
    
    // Check if we're in a rebase state
    if git.is_in_rebase()? {
        print_error("Repository is in a rebase state.");
        println!("\nUse {} to continue after resolving conflicts.", "dotfiles sync --continue".cyan().bold());
        bail!("In rebase state");
    }
    
    // Clean up any temporary conflict files from previous runs
    cleanup_temp_dir(&repo_path)?;

    // --- SETUP ---
    let tracked = manager.load_tracked_files()?.clone();
    
    if tracked.is_empty() {
        print_info("No files to track. Use 'dotfiles add' to add files.");
        return Ok(());
    }

    // Always sync all files - check if any are encrypted
    let has_encrypted = tracked.iter().any(|f| f.encrypted);
    let encryption_key = if has_encrypted {
        Some(resolve_encryption_key(&repo_path)?)
    } else {
        None
    };

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
    sync_home_to_repo(&manager, &tracked, encryption_key.as_ref())?;

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
                
                // Get conflicted files and decrypt if needed
                if let Ok(conflicted_files) = git.get_conflicted_files() {
                    println!("\n{}", "Conflicted files:".yellow().bold());
                    
                    let encryption_key = if tracked.iter().any(|f| f.encrypted) {
                        resolve_encryption_key(&repo_path).ok()
                    } else {
                        None
                    };
                    
                    for file in &conflicted_files {
                        println!("  {} {}", "✗".red(), file);
                        
                        // If it's an encrypted file, decrypt to temp for easier conflict resolution
                        if file.ends_with(".enc") {
                            if let Some(key) = encryption_key.as_ref() {
                                let full_path = repo_path.join(file);
                                if let Err(decrypt_err) = decrypt_to_temp(&repo_path, &full_path, key) {
                                    print_warning(&format!("Could not decrypt {}: {}", file, decrypt_err));
                                }
                            }
                        }
                    }
                    
                    if encryption_key.is_some() {
                        println!("\n{}", "Encrypted files have been decrypted to:".yellow());
                        println!("  {}", repo_path.join(TEMP_CONFLICTS_DIR).display());
                    }
                }
                
                println!("\n{}", "To resolve:".yellow().bold());
                println!("  1. Resolve conflicts in the files listed above");
                println!("  2. Run {} to continue", "dotfiles sync --continue".cyan().bold());
                
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
    let backup_created = backup_home_files(&repo_path, &tracked)?;
    if backup_created {
        print_success("Backup created");
    } else {
        print_info("No files to backup (first sync or files don't exist)");
    }
    
    print_info("Step 5/6: Exporting to Home directory...");
    sync_repo_to_home(&manager, &tracked, encryption_key.as_ref())?;

    // Note: Backups are local-only (in .gitignore), not committed

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

fn decrypt_to_temp(repo_path: &Path, encrypted_file: &Path, key: &[u8; 32]) -> Result<()> {
    let temp_dir = repo_path.join(TEMP_CONFLICTS_DIR);
    std::fs::create_dir_all(&temp_dir)?;
    
    let rel_path = encrypted_file.strip_prefix(repo_path)
        .context("Failed to get relative path")?;
    
    let decrypted_name = rel_path.with_extension("");
    let decrypted_path = temp_dir.join(decrypted_name);
    
    if let Some(parent) = decrypted_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    
    // Check if this is a conflicted file by trying to extract both versions
    let git = GitRepo::new(repo_path);
    let file_path_str = rel_path.to_string_lossy();
    
    let has_ours = git.get_file_version(&file_path_str, 2).is_ok();
    let has_theirs = git.get_file_version(&file_path_str, 3).is_ok();
    
    if has_ours && has_theirs {
        // This is a conflicted encrypted file - extract both versions and create merged file with conflict markers
        let ours_encrypted = git.get_file_version(&file_path_str, 2)?;
        let theirs_encrypted = git.get_file_version(&file_path_str, 3)?;
        
        // Write encrypted versions to temp files
        let temp_ours_enc = std::env::temp_dir().join(format!("dotfiles_ours_{}", uuid::Uuid::new_v4()));
        let temp_theirs_enc = std::env::temp_dir().join(format!("dotfiles_theirs_{}", uuid::Uuid::new_v4()));
        std::fs::write(&temp_ours_enc, &ours_encrypted)?;
        std::fs::write(&temp_theirs_enc, &theirs_encrypted)?;
        
        // Decrypt both versions
        let temp_ours_dec = std::env::temp_dir().join(format!("dotfiles_ours_dec_{}", uuid::Uuid::new_v4()));
        let temp_theirs_dec = std::env::temp_dir().join(format!("dotfiles_theirs_dec_{}", uuid::Uuid::new_v4()));
        
        FileEncryptor::decrypt_file(&temp_ours_enc, &temp_ours_dec, key)?;
        FileEncryptor::decrypt_file(&temp_theirs_enc, &temp_theirs_dec, key)?;
        
        // Read decrypted content
        let ours_content = std::fs::read_to_string(&temp_ours_dec)
            .unwrap_or_else(|_| String::from("<binary content>"));
        let theirs_content = std::fs::read_to_string(&temp_theirs_dec)
            .unwrap_or_else(|_| String::from("<binary content>"));
        
        // Create merged file with conflict markers
        let merged_content = format!(
            "<<<<<<< HEAD (ours - current)\n{}=======\n{}>>>>>>> theirs (incoming)\n",
            ours_content,
            theirs_content
        );
        
        std::fs::write(&decrypted_path, merged_content)?;
        
        // Clean up temp files
        let _ = std::fs::remove_file(temp_ours_enc);
        let _ = std::fs::remove_file(temp_theirs_enc);
        let _ = std::fs::remove_file(temp_ours_dec);
        let _ = std::fs::remove_file(temp_theirs_dec);
        
        print_info(&format!("Decrypted conflicted file with markers to: {}", decrypted_path.display()));
    } else {
        // Not conflicted or can't extract versions - just decrypt as-is
        FileEncryptor::decrypt_file(encrypted_file, &decrypted_path, key)?;
        print_info(&format!("Decrypted to: {}", decrypted_path.display()));
    }
    
    Ok(())
}

fn cleanup_temp_dir(repo_path: &Path) -> Result<()> {
    let temp_dir = repo_path.join(TEMP_CONFLICTS_DIR);
    
    if temp_dir.exists() {
        std::fs::remove_dir_all(&temp_dir).ok();
    }
    
    Ok(())
}

fn resolve_encryption_key(repo_path: &Path) -> Result<[u8; 32]> {
    let has_marker = FileEncryptor::is_encryption_setup(repo_path);
    let has_key = FileEncryptor::has_local_key();
    
    if has_marker && has_key {
        // Load existing key from home directory
        FileEncryptor::load_key_from_home()
    } else if has_marker && !has_key {
        // Marker exists but no key - need seed phrase
        print_info("Encrypted files detected but encryption key not found in home directory.");
        print_info("Please enter your 12-word seed phrase to restore encryption.");
        
        let mnemonic = FileEncryptor::prompt_for_seed_phrase()?;
        let key = FileEncryptor::derive_key_from_mnemonic(&mnemonic);
        FileEncryptor::save_key_to_home(&key)?;
        print_success("Encryption key restored and saved to home directory");
        
        Ok(key)
    } else if !has_marker && check_for_encrypted_files_in_repo(repo_path) {
        // Old repo without marker but has encrypted files
        print_warning("Encrypted files detected but no encryption marker file.");
        print_info("Please enter your 12-word seed phrase to restore encryption.");
        
        let mnemonic = FileEncryptor::prompt_for_seed_phrase()?;
        let key = FileEncryptor::derive_key_from_mnemonic(&mnemonic);
        FileEncryptor::save_key_to_home(&key)?;
        FileEncryptor::create_encryption_marker(repo_path)?;
        print_success("Encryption key restored and marker file created");
        
        Ok(key)
    } else {
        bail!("No encrypted files found. Use 'dotfiles add --encrypt <file>' to add encrypted files.");
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

fn backup_home_files(repo_path: &std::path::Path, files: &[TrackedFile]) -> Result<bool> {
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
        
        // Skip directories - we only backup files
        if home_path.is_dir() {
            continue;
        }
        
        // Create backup path mirroring the home structure
        let relative_path = file.path.trim_start_matches("~/").trim_start_matches('/');
        let backup_file = backup_dir.join(relative_path);
        
        // Create parent directory
        if let Some(parent) = backup_file.parent() {
            fs::create_dir_all(parent)?;
        }
        
        // IMPORTANT: Backups are ALWAYS stored UNENCRYPTED locally
        // This is safe because .backup/ is in .gitignore (never pushed to remote)
        // This allows emergency recovery without needing seed phrase
        FileSyncer::sync_file(&home_path, &backup_file)?;
        any_backed_up = true;
    }
    
    // If no files were backed up, remove the empty directory
    if !any_backed_up && backup_dir.exists() {
        fs::remove_dir_all(&backup_dir).ok();
    }
    
    Ok(any_backed_up)
}

fn sync_home_to_repo(manager: &ConfigManager, files: &[TrackedFile], encryption_key: Option<&[u8; 32]>) -> Result<()> {
    let repo_path = manager.get_repo_path();
    let mut synced_count = 0;

    for file in files {
        let home_path = FileSyncer::expand_tilde(&file.path);
        
        if !home_path.exists() {
            continue;
        }
        
        // Skip directories - we only sync files
        if home_path.is_dir() {
            continue;
        }
        
        let repo_file = repo_path.join(file.path.trim_start_matches("~/").trim_start_matches('/'));

        if file.encrypted {
            if let Some(key) = encryption_key {
                let encrypted_path = repo_file.with_extension("enc");
                
                // Check if file needs syncing (decrypt existing and compare plaintext)
                let needs_sync = if encrypted_path.exists() {
                    // Decrypt existing encrypted file to temp and compare with source
                    let temp_decrypted = std::env::temp_dir().join(format!("dotfiles_temp_{}", uuid::Uuid::new_v4()));
                    FileEncryptor::decrypt_file(&encrypted_path, &temp_decrypted, key)?;
                    let is_different = !files_are_identical(&home_path, &temp_decrypted)?;
                    let _ = std::fs::remove_file(temp_decrypted);
                    is_different
                } else {
                    true
                };
                
                if needs_sync {
                    FileEncryptor::encrypt_file(&home_path, &encrypted_path, key)?;
                    synced_count += 1;
                }
            }
        } else {
            // Check if non-encrypted file needs syncing
            let needs_sync = if repo_file.exists() {
                !files_are_identical(&home_path, &repo_file)?
            } else {
                true
            };
            
            if needs_sync {
                FileSyncer::sync_file(&home_path, &repo_file)?;
                synced_count += 1;
            }
        }
    }
    
    if synced_count > 0 {
        print_info(&format!("Synced {} file(s) with changes", synced_count));
    } else {
        print_info("All files already in sync (no changes)");
    }
    
    Ok(())
}

fn files_are_identical(path1: &std::path::Path, path2: &std::path::Path) -> Result<bool> {
    use std::io::Read;
    
    let mut file1 = std::fs::File::open(path1)?;
    let mut file2 = std::fs::File::open(path2)?;
    
    let mut buf1 = Vec::new();
    let mut buf2 = Vec::new();
    
    file1.read_to_end(&mut buf1)?;
    file2.read_to_end(&mut buf2)?;
    
    Ok(buf1 == buf2)
}

fn sync_repo_to_home(manager: &ConfigManager, files: &[TrackedFile], encryption_key: Option<&[u8; 32]>) -> Result<()> {
    let repo_path = manager.get_repo_path();
    let mut synced_count = 0;

    for file in files {
        let home_path = FileSyncer::expand_tilde(&file.path);
        let repo_file = repo_path.join(file.path.trim_start_matches("~/").trim_start_matches('/'));

        // Skip directories - we only sync files
        if repo_file.exists() && repo_file.is_dir() {
            continue;
        }

        if file.encrypted {
            if let Some(key) = encryption_key {
                let encrypted_path = repo_file.with_extension("enc");
                if encrypted_path.exists() {
                    // Create parent directory if it doesn't exist
                    if let Some(parent) = home_path.parent() {
                        std::fs::create_dir_all(parent)?;
                    }
                    
                    // Check if decryption is needed (compare decrypted content)
                    let needs_sync = if home_path.exists() {
                        // Decrypt to temp and compare
                        let temp_decrypted = std::env::temp_dir().join(format!("dotfiles_temp_{}", uuid::Uuid::new_v4()));
                        FileEncryptor::decrypt_file(&encrypted_path, &temp_decrypted, key)?;
                        let is_different = !files_are_identical(&temp_decrypted, &home_path)?;
                        let _ = std::fs::remove_file(temp_decrypted);
                        is_different
                    } else {
                        true
                    };
                    
                    if needs_sync {
                        FileEncryptor::decrypt_file(&encrypted_path, &home_path, key)?;
                        synced_count += 1;
                    }
                }
            }
        } else if repo_file.exists() {
            // Create parent directory if it doesn't exist
            if let Some(parent) = home_path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            
            // Check if file needs syncing
            let needs_sync = if home_path.exists() {
                !files_are_identical(&repo_file, &home_path)?
            } else {
                true
            };
            
            if needs_sync {
                FileSyncer::sync_file(&repo_file, &home_path)?;
                synced_count += 1;
            }
        }
    }
    
    if synced_count > 0 {
        print_info(&format!("Exported {} file(s) with changes", synced_count));
    } else {
        print_info("All files already in sync (no changes)");
    }
    
    Ok(())
}
