use anyhow::{bail, Context, Result};
use crate::config::ConfigManager;
use crate::utils::{print_error, print_success};
use std::path::PathBuf;

pub fn execute(key: String, value: String) -> Result<()> {
    let repo_path = ConfigManager::resolve_repo_path()?;
    let manager = ConfigManager::new(repo_path.clone());
    
    if !manager.is_initialized() {
        print_error("Not in a dotfiles repository. Run 'dotfiles init' first.");
        bail!("Repository not initialized");
    }
    
    // Load current config (will merge local and repo configs)
    let mut config = manager.load_config()?;
    
    match key.as_str() {
        "repo_path" => {
            let path = PathBuf::from(&value);
            let canonical = path.canonicalize()
                .context(format!("Failed to resolve path: {}", value))?;
            config.repo_path = canonical.clone();
            manager.save_local_config(canonical)?;
            print_success(&format!("Set repo_path to: {}", value));
        },
        "use_xdg" => {
            let use_xdg = value.parse::<bool>()
                .context("Invalid boolean value. Use 'true' or 'false'")?;
            config.use_xdg = use_xdg;
            save_to_local_config(&manager, &config)?;
            print_success(&format!("Set use_xdg to: {}", use_xdg));
        },
        "encryption_key_path" => {
            let path = PathBuf::from(&value);
            let canonical = path.canonicalize()
                .context(format!("Failed to resolve path: {}", value))?;
            config.encryption_key_path = Some(canonical);
            save_to_local_config(&manager, &config)?;
            print_success(&format!("Set encryption_key_path to: {}", value));
        },
        "tag" => {
            config.tag = if value.is_empty() { None } else { Some(value.clone()) };
            save_to_local_config(&manager, &config)?;
            print_success(&format!("Set tag to: {}", value));
        },
        _ => {
            print_error(&format!("Unknown config key: {}", key));
            println!("\nAvailable keys:");
            println!("  - repo_path");
            println!("  - use_xdg");
            println!("  - encryption_key_path");
            println!("  - tag");
            bail!("Invalid config key");
        }
    }
    
    Ok(())
}

fn save_to_local_config(manager: &ConfigManager, config: &crate::config::DotfilesConfig) -> Result<()> {
    let local_config_path = manager.get_local_config_path();
    
    // Create a minimal local config with just the settings we want to persist
    let local_config = crate::config::DotfilesConfig {
        use_xdg: config.use_xdg,
        repo_path: config.repo_path.clone(),
        home_path: config.home_path.clone(),
        encryption_key_path: config.encryption_key_path.clone(),
        tag: config.tag.clone(),
        tracked_files: None,
    };
    
    let content = serde_json::to_string_pretty(&local_config)
        .context("Failed to serialize local config")?;
    
    std::fs::write(&local_config_path, content)
        .context("Failed to write local config file")?;
    
    Ok(())
}
