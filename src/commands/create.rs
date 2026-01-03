use anyhow::{bail, Result};
use crate::config::ConfigManager;
use crate::db::ConfigDatabase;
use crate::utils::{print_error, print_info, print_success};

pub fn execute(stub: String, paths: Vec<String>, tag: Option<String>) -> Result<()> {
    let repo_path = ConfigManager::resolve_repo_path()?;
    let manager = ConfigManager::new(repo_path.clone());

    if !manager.is_initialized() {
        print_error("Not in a dotfiles repository. Run 'dotfiles init' first.");
        bail!("Repository not initialized");
    }

    if paths.is_empty() {
        print_error("No paths provided");
        print_info("Usage: dotfiles create <stub> <path1> [path2] ...");
        print_info("Example: dotfiles create myapp ~/.myapprc ~/.config/myapp/config");
        bail!("No paths provided");
    }

    let config = manager.load_config()?;
    let tag_to_use = tag.or(config.tag.clone());
    let db = ConfigDatabase::new_with_tag(&repo_path, tag_to_use.as_deref());

    if db.load_stub(&stub)?.is_some() {
        print_error(&format!("Stub '{}' already exists", stub));
        bail!("Stub already exists");
    }

    print_info(&format!("Creating stub '{}'...", stub));

    let name = stub
        .split('-')
        .map(|s| {
            let mut c = s.chars();
            match c.next() {
                None => String::new(),
                Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ");

    db.create_stub(&stub, &name, &paths)?;

    print_success(&format!("Created custom stub '{}' with {} paths", stub, paths.len()));
    for path in &paths {
        println!("  • {}", path);
    }
    
    if let Some(t) = tag_to_use {
        print_info(&format!("Tagged as: {}", t));
    }

    println!();
    print_info(&format!("Use 'dotfiles add {}' to start tracking these files", stub));

    Ok(())
}
