use anyhow::{bail, Result};
use crate::config::ConfigManager;
use crate::utils::{print_success, print_error, print_warning};

pub fn execute(stub: String) -> Result<()> {
    let repo_path = std::env::current_dir()?;
    let manager = ConfigManager::new(repo_path);

    if !manager.is_initialized() {
        print_error("Not in a dotfiles repository. Run 'dotfiles init' first.");
        bail!("Repository not initialized");
    }

    let mut tracked = manager.load_tracked_files()?;
    let original_len = tracked.len();

    tracked.retain(|t| t.stub.as_deref() != Some(stub.as_str()) && t.path != stub);

    if tracked.len() == original_len {
        print_warning(&format!("Stub '{}' was not being tracked", stub));
        return Ok(());
    }

    manager.save_tracked_files(&tracked)?;
    print_success(&format!("Removed stub '{}' from tracking", stub));
    print_warning("Note: Files remain in repository. Commit changes if needed.");

    Ok(())
}
