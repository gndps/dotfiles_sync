use anyhow::{bail, Result};
use crate::config::ConfigManager;
use crate::git::GitRepo;
use crate::sync::FileSyncer;
use crate::utils::{print_error, print_info, print_success};

pub fn execute() -> Result<()> {
    let repo_path = std::env::current_dir()?;
    let manager = ConfigManager::new(repo_path.clone());

    if !manager.is_initialized() {
        print_error("Not in a dotfiles repository. Run 'dotfiles init' first.");
        bail!("Repository not initialized");
    }

    let git = GitRepo::new(&repo_path);

    if git.is_in_merge()? {
        print_error("Repository is in the middle of a merge conflict!");
        print_info("Resolve conflicts before syncing to home directory");
        bail!("Merge in progress");
    }

    print_info("Syncing from repository to home directory...");

    let tracked = manager.load_tracked_files()?;

    for file in &tracked {
        let home_path = FileSyncer::expand_tilde(&file.path);
        let repo_file = repo_path.join(file.path.trim_start_matches("~/"));

        if repo_file.exists() {
            FileSyncer::sync_file(&repo_file, &home_path)?;
            print_success(&format!("Synced: {}", file.path));
        } else {
            print_info(&format!("Not in repo (skipped): {}", file.path));
        }
    }

    println!();
    print_success(&format!("Synced {} files to home directory", tracked.len()));

    Ok(())
}
