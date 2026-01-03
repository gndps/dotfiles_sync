use anyhow::{bail, Result};
use colored::Colorize;
use crate::config::{ConfigManager, TrackedFile};
use crate::git::GitRepo;
use crate::sync::FileSyncer;
use crate::utils::{print_error, print_info, print_success, print_warning};

pub fn execute(dir: Option<std::path::PathBuf>) -> Result<()> {
    use anyhow::Context;
    
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
        ConfigManager::resolve_repo_path()?
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
    let tracked = manager.load_tracked_files()?.clone();
    
    if tracked.is_empty() {
        print_info("No files to track. Use 'dotfiles add' to add files.");
        return Ok(());
    }

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
    sync_home_to_repo(&manager, &tracked)?;

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
    let backup_created = backup_home_files(&repo_path, &tracked)?;
    if backup_created {
        print_success("Backup created");
    } else {
        print_info("No files to backup (first sync or files don't exist)");
    }
    
    print_info("Step 5/6: Exporting to Home directory...");
    sync_repo_to_home(&manager, &tracked)?;

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
        
        // IMPORTANT: Backups are stored locally only
        // This is safe because .backup/ is in .gitignore (never pushed to remote)
        // This allows emergency recovery
        FileSyncer::sync_file(&home_path, &backup_file)?;
        any_backed_up = true;
    }
    
    // If no files were backed up, remove the empty directory
    if !any_backed_up && backup_dir.exists() {
        fs::remove_dir_all(&backup_dir).ok();
    }
    
    Ok(any_backed_up)
}

fn sync_home_to_repo(manager: &ConfigManager, files: &[TrackedFile]) -> Result<()> {
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

        // Check if file needs syncing
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

fn sync_repo_to_home(manager: &ConfigManager, files: &[TrackedFile]) -> Result<()> {
    let repo_path = manager.get_repo_path();
    let mut synced_count = 0;

    for file in files {
        let home_path = FileSyncer::expand_tilde(&file.path);
        let repo_file = repo_path.join(file.path.trim_start_matches("~/").trim_start_matches('/'));

        // Skip directories - we only sync files
        if repo_file.exists() && repo_file.is_dir() {
            continue;
        }

        if repo_file.exists() {
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
