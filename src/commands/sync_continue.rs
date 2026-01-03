use anyhow::{bail, Context, Result};
use colored::Colorize;
use crate::config::{ConfigManager, TrackedFile};
use crate::encryption::FileEncryptor;
use crate::git::GitRepo;
use crate::utils::{print_error, print_info, print_success, print_warning};
use std::path::Path;

const TEMP_CONFLICTS_DIR: &str = ".dotfiles_conflicts_temp";

pub fn execute() -> Result<()> {
    let repo_path = ConfigManager::resolve_repo_path()?;
    let manager = ConfigManager::new(repo_path.clone());
    let git = GitRepo::new(&repo_path);

    if !manager.is_initialized() {
        print_error("Not in a dotfiles repository. Run 'dotfiles init' first.");
        bail!("Repository not initialized");
    }

    if !git.is_repo() {
        print_error("Not a git repository.");
        bail!("Not a git repository");
    }

    // Check if we're in a rebase state
    if !git.is_in_rebase()? {
        print_error("Not in a rebase state. Use 'dotfiles sync' for normal syncing.");
        bail!("Not in rebase state");
    }

    print_info("Continuing sync after conflict resolution...");

    // Get tracked files
    let tracked = manager.load_tracked_files()?;
    
    // Check if there are any conflicts
    if git.has_conflicts()? {
        let conflicted_files = git.get_conflicted_files()?;
        
        print_error("There are still unresolved conflicts!");
        println!("\n{}", "Conflicted files:".yellow().bold());
        
        // Check if any conflicted files are encrypted
        let encryption_key = get_encryption_key_if_needed(&repo_path, &tracked)?;
        
        for file in &conflicted_files {
            let full_path = repo_path.join(file);
            
            // Check if this is an encrypted file
            if file.ends_with(".enc") {
                println!("  {} {}", "✗".red(), file);
                
                // Decrypt to temp folder for conflict resolution
                if let Some(key) = encryption_key.as_ref() {
                    decrypt_to_temp(&repo_path, &full_path, key)?;
                }
            } else {
                println!("  {} {}", "✗".red(), file);
            }
        }
        
        if encryption_key.is_some() {
            println!("\n{}", "Encrypted files have been decrypted to:".yellow());
            println!("  {}", repo_path.join(TEMP_CONFLICTS_DIR).display());
            println!("\nResolve conflicts in the decrypted files, then:");
            println!("  1. The changes will be encrypted back automatically");
        }
        
        println!("\n{}", "After resolving all conflicts, run:".yellow());
        println!("  {}", "dotfiles sync --continue".cyan().bold());
        
        bail!("Conflicts must be resolved before continuing");
    }

    // Check for conflict markers in all files
    print_info("Checking for conflict markers...");
    let files_with_markers = check_for_conflict_markers(&repo_path, &tracked)?;
    
    if !files_with_markers.is_empty() {
        print_error("Found conflict markers in files!");
        println!("\n{}", "Files with conflict markers:".yellow().bold());
        
        for file in &files_with_markers {
            println!("  {} {}", "⚠".red(), file);
        }
        
        println!("\n{}", "Please resolve all conflict markers (<<<<<<, ======, >>>>>>)".yellow());
        bail!("Conflict markers found");
    }

    // Process temp decrypted files if they exist
    process_temp_conflicts(&repo_path, &tracked)?;

    // Add all files
    print_info("Adding resolved files...");
    git.add_all()?;

    // Continue rebase
    print_info("Continuing rebase...");
    match git.rebase_continue() {
        Ok(_) => {
            print_success("Rebase continued successfully!");
            
            // Clean up temp directory
            cleanup_temp_dir(&repo_path)?;
            
            println!("\n{}", "Next steps:".bold());
            println!("  Run {}", format!("dotfiles sync").cyan());
        },
        Err(e) => {
            print_warning(&format!("Rebase continue failed: {}", e));
            println!("\n{}", "This might mean:".yellow());
            println!("  1. There are more conflicts to resolve");
            println!("  2. Run {}", format!("dotfiles sync --continue").cyan());
            return Err(e);
        }
    }

    Ok(())
}

fn get_encryption_key_if_needed(repo_path: &Path, tracked: &[TrackedFile]) -> Result<Option<[u8; 32]>> {
    let has_encrypted = tracked.iter().any(|f| f.encrypted);
    
    if has_encrypted {
        let has_marker = FileEncryptor::is_encryption_setup(repo_path);
        let has_key = FileEncryptor::has_local_key();
        
        if has_marker && has_key {
            Ok(Some(FileEncryptor::load_key_from_home()?))
        } else if has_marker && !has_key {
            print_info("Encrypted files detected. Please enter your seed phrase.");
            let mnemonic = FileEncryptor::prompt_for_seed_phrase()?;
            let key = FileEncryptor::derive_key_from_mnemonic(&mnemonic);
            FileEncryptor::save_key_to_home(&key)?;
            Ok(Some(key))
        } else {
            Ok(None)
        }
    } else {
        Ok(None)
    }
}

fn decrypt_to_temp(repo_path: &Path, encrypted_file: &Path, key: &[u8; 32]) -> Result<()> {
    let temp_dir = repo_path.join(TEMP_CONFLICTS_DIR);
    std::fs::create_dir_all(&temp_dir)?;
    
    // Get relative path
    let rel_path = encrypted_file.strip_prefix(repo_path)
        .context("Failed to get relative path")?;
    
    // Remove .enc extension for decrypted file
    let decrypted_name = rel_path.with_extension("");
    let decrypted_path = temp_dir.join(decrypted_name);
    
    // Create parent directories
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

fn check_for_conflict_markers(repo_path: &Path, tracked: &[TrackedFile]) -> Result<Vec<String>> {
    let mut files_with_markers = Vec::new();
    
    for file in tracked {
        let repo_file = repo_path.join(file.path.trim_start_matches("~/").trim_start_matches('/'));
        
        // For encrypted files, ONLY check temp decrypted versions (not the encrypted file itself)
        if file.encrypted {
            let temp_path = repo_path.join(TEMP_CONFLICTS_DIR).join(file.path.trim_start_matches("~/").trim_start_matches('/'));
            if temp_path.exists() {
                if file_has_conflict_markers(&temp_path)? {
                    files_with_markers.push(file.path.clone());
                }
            }
        } else if repo_file.exists() {
            // For unencrypted files, check the file directly
            if file_has_conflict_markers(&repo_file)? {
                files_with_markers.push(file.path.clone());
            }
        }
    }
    
    Ok(files_with_markers)
}

fn file_has_conflict_markers(path: &Path) -> Result<bool> {
    let content = std::fs::read_to_string(path)?;
    Ok(content.contains("<<<<<<<") || content.contains("=======") || content.contains(">>>>>>>"))
}

fn process_temp_conflicts(repo_path: &Path, tracked: &[TrackedFile]) -> Result<()> {
    let temp_dir = repo_path.join(TEMP_CONFLICTS_DIR);
    
    if !temp_dir.exists() {
        return Ok(());
    }
    
    print_info("Processing temporary decrypted files...");
    
    // Get encryption key if needed
    let encryption_key = get_encryption_key_if_needed(repo_path, tracked)?;
    
    for file in tracked {
        if file.encrypted {
            let temp_path = temp_dir.join(file.path.trim_start_matches("~/"));
            
            if temp_path.exists() {
                // Encrypt back to repo
                if let Some(key) = encryption_key.as_ref() {
                    let repo_file = repo_path.join(file.path.trim_start_matches("~/").trim_start_matches('/'));
                    let encrypted_path = repo_file.with_extension("enc");
                    
                    FileEncryptor::encrypt_file(&temp_path, &encrypted_path, key)?;
                    print_success(&format!("Re-encrypted: {}", file.path));
                }
            }
        }
    }
    
    Ok(())
}

fn cleanup_temp_dir(repo_path: &Path) -> Result<()> {
    let temp_dir = repo_path.join(TEMP_CONFLICTS_DIR);
    
    if temp_dir.exists() {
        std::fs::remove_dir_all(&temp_dir)
            .context("Failed to clean up temporary directory")?;
        print_info("Cleaned up temporary conflict files");
    }
    
    Ok(())
}
