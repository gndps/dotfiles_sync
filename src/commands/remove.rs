use anyhow::{bail, Result};
use crate::config::ConfigManager;
use crate::sync::FileSyncer;
use crate::utils::{print_success, print_error, print_warning};

pub fn execute(stub_or_path: String) -> Result<()> {
    let repo_path = std::env::current_dir()?;
    let manager = ConfigManager::new(repo_path);

    if !manager.is_initialized() {
        print_error("Not in a dotfiles repository. Run 'dotfiles init' first.");
        bail!("Repository not initialized");
    }

    let mut tracked = manager.load_tracked_files()?;
    let original_len = tracked.len();

    // Check if it's a path or a stub
    let is_path = stub_or_path.contains('/') || stub_or_path.starts_with('~') || stub_or_path.starts_with('.');
    
    if is_path {
        // Normalize the path to match how it's stored
        let normalized_path = normalize_path(&stub_or_path);
        tracked.retain(|t| t.path != normalized_path);
        
        if tracked.len() == original_len {
            print_warning(&format!("Path '{}' was not being tracked", stub_or_path));
            return Ok(());
        }
        
        manager.save_tracked_files(&tracked)?;
        print_success(&format!("Removed '{}' from tracking", stub_or_path));
    } else {
        // It's a stub name
        tracked.retain(|t| t.stub.as_deref() != Some(stub_or_path.as_str()));
        
        if tracked.len() == original_len {
            print_warning(&format!("Stub '{}' was not being tracked", stub_or_path));
            return Ok(());
        }
        
        manager.save_tracked_files(&tracked)?;
        print_success(&format!("Removed stub '{}' from tracking", stub_or_path));
    }
    
    print_warning("Note: Files remain in repository. Commit changes if needed.");

    Ok(())
}

fn normalize_path(path: &str) -> String {
    // Normalize path to start with ~/
    if let Some(home) = dirs::home_dir() {
        let expanded = FileSyncer::expand_tilde(path);
        if let Ok(rel) = expanded.strip_prefix(&home) {
            return format!("~/{}", rel.display());
        }
    }
    path.to_string()
}
