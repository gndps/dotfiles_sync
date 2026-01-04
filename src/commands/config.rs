use anyhow::Result;
use colored::Colorize;
use crate::config::ConfigManager;
use crate::utils::{print_success, print_info};

pub fn execute_set(field: String, value: String) -> Result<()> {
    let repo_path = ConfigManager::resolve_repo_path()?;
    let manager = ConfigManager::new(repo_path);
    
    manager.update_local_config_field(&field, &value)?;
    
    print_success(&format!("Set {} = {}", field.green(), value.cyan()));
    Ok(())
}

pub fn execute_show() -> Result<()> {
    let repo_path = ConfigManager::resolve_repo_path()?;
    let manager = ConfigManager::new(repo_path);
    
    let local_config = manager.load_local_config()?;
    
    println!("\n{}", "Local Configuration:".bold().cyan());
    println!("  {} {}", "use_xdg:".bold(), local_config.use_xdg);
    println!("  {} {}", "repo_path:".bold(), local_config.repo_path.display());
    println!("  {} {}", "home_path:".bold(), local_config.home_path.display());
    print!("  {} ", "tag:".bold());
    if let Some(tag) = local_config.tag {
        println!("{}", tag);
    } else {
        println!("{}", "None".dimmed());
    }
    
    print_info(&format!("\nConfig file: {}", manager.get_local_config_path().display()));
    
    Ok(())
}
