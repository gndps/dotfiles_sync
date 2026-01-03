use anyhow::{Context, Result};
use std::fs;
use std::path::PathBuf;
use crate::config::{ConfigManager, DotfilesConfig};
use crate::git::GitRepo;
use crate::utils::{print_success, print_info};

pub fn execute(path: Option<PathBuf>, tag: Option<String>) -> Result<()> {
    let repo_path = path.unwrap_or_else(|| PathBuf::from("."));
    
    if !repo_path.exists() {
        fs::create_dir_all(&repo_path)
            .context("Failed to create directory")?;
    }

    let canonical_repo_path = repo_path.canonicalize()
        .unwrap_or_else(|_| repo_path.clone());
    
    let manager = ConfigManager::new(canonical_repo_path.clone());
    
    if manager.is_initialized() {
        print_info("Dotfiles repository already initialized");
        return Ok(());
    }

    print_info("Initializing dotfiles repository...");

    let mut config = DotfilesConfig::default();
    config.repo_path = canonical_repo_path.clone();
    config.tag = tag.clone();
    
    if let Some(home) = dirs::home_dir() {
        config.home_path = home;
    }

    manager.save_config(&config)
        .context("Failed to save config")?;
    print_success("Created dotfiles.config.json");
    
    // Save repo path to local config in home directory
    manager.save_local_config(canonical_repo_path.clone())?;
    print_success(&format!("Saved dotfiles directory to local config: {}", canonical_repo_path.display()));
    
    if let Some(ref t) = tag {
        print_info(&format!("Using tag: {}", t));
    }

    let custom_path = if let Some(ref t) = tag {
        repo_path.join("custom_db").join(t)
    } else {
        repo_path.join("custom_db")
    };
    
    fs::create_dir_all(custom_path.join("applications"))?;
    fs::create_dir_all(custom_path.join("default_configs"))?;
    print_success("Created custom_db directory structure");

    let git = GitRepo::new(&repo_path);
    if !git.is_repo() {
        git.init().context("Failed to initialize git repository")?;
        print_success("Initialized git repository");
    } else {
        print_info("Git repository already exists");
    }

    // Create .gitignore
    let gitignore_path = repo_path.join(".gitignore");
    let mut gitignore_content = String::new();
    
    if gitignore_path.exists() {
        gitignore_content = fs::read_to_string(&gitignore_path)
            .context("Failed to read .gitignore")?;
    }
    
    let mut updated = false;
    
    if !gitignore_content.contains("dotfiles.local.config.json") {
        gitignore_content.push_str("\n# Dotfiles local configuration\ndotfiles.local.config.json\n");
        updated = true;
    }
    
    // Ensure backups stay local
    if !gitignore_content.contains(".backup/") {
        gitignore_content.push_str("\n# Local backups (for emergency recovery)\n.backup/\n");
        updated = true;
    }
    
    if updated {
        fs::write(&gitignore_path, gitignore_content)
            .context("Failed to write .gitignore")?;
        print_success("Updated .gitignore");
    }

    // Make initial commit
    if git.is_dirty()? {
        git.add_all()?;
        git.commit("Initial commit: dotfiles setup")?;
        print_success("Created initial commit");
    }

    println!();
    print_success("Dotfiles repository initialized successfully!");
    print_info("Next steps:");
    println!("  1. Add config files: dotfiles add <stub>");
    println!("  2. Or add direct paths: dotfiles add ~/.vimrc");
    println!("  3. Sync your files: dotfiles sync");
    println!("\nTip: Use 'dotfiles list --all' to see 500+ available stubs");

    Ok(())
}
