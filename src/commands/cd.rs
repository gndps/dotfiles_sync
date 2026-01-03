use anyhow::{bail, Result};
use crate::config::ConfigManager;
use crate::utils::print_error;

pub fn execute() -> Result<()> {
    // Resolve repo path from local config or current directory
    let repo_path = ConfigManager::resolve_repo_path()?;
    let manager = ConfigManager::new(repo_path.clone());
    
    if !manager.is_initialized() {
        print_error("Not in a dotfiles repository and no local config found.");
        print_error("Run 'dotfiles init' in your dotfiles directory first.");
        bail!("Repository not found");
    }
    
    // Output the dotfiles directory path
    println!("{}", repo_path.display());
    Ok(())
}
