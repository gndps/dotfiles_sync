use anyhow::{bail, Result};
use crate::config::ConfigManager;
use crate::utils::print_error;

pub fn execute() -> Result<()> {
    // Check if we're already in a dotfiles repository
    let current_dir = std::env::current_dir()?;
    let manager = ConfigManager::new(current_dir);
    
    if manager.is_initialized() {
        // Already in dotfiles directory
        println!("{}", manager.get_repo_path().display());
        return Ok(());
    }
    
    // Try to load config from home directory
    if let Some(home) = dirs::home_dir() {
        let local_config_path = home.join(".dotfiles.local.config.json");
        if local_config_path.exists() {
            if let Ok(content) = std::fs::read_to_string(&local_config_path) {
                if let Ok(config) = serde_json::from_str::<crate::config::DotfilesConfig>(&content) {
                    println!("{}", config.repo_path.display());
                    return Ok(());
                }
            }
        }
    }
    
    print_error("Not in a dotfiles repository and no local config found.");
    print_error("Run 'dotfiles init' in your dotfiles directory first.");
    bail!("Repository not found");
}
