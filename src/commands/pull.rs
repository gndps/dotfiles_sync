use anyhow::{bail, Result};
use crate::config::ConfigManager;
use crate::git::GitRepo;
use crate::utils::{print_error, print_info, print_success, print_warning};

pub fn execute() -> Result<()> {
    let repo_path = std::env::current_dir()?;
    let manager = ConfigManager::new(repo_path.clone());

    if !manager.is_initialized() {
        print_error("Not in a dotfiles repository. Run 'dotfiles init' first.");
        bail!("Repository not initialized");
    }

    let git = GitRepo::new(&repo_path);

    if !git.is_repo() {
        print_error("Not a git repository. Initialize git first.");
        bail!("Not a git repository");
    }

    if git.is_in_merge()? {
        print_error("Repository is in the middle of a merge conflict!");
        print_info("Complete the merge before pulling");
        bail!("Merge in progress");
    }

    if !git.has_remote()? {
        print_warning("No remote repository configured");
        print_info("Add a remote with: git remote add origin <url>");
        return Ok(());
    }

    print_info("Pulling from remote repository...");

    let branch = git.get_current_branch()?;
    git.pull("origin", &branch)?;

    print_success("Pull completed successfully");

    Ok(())
}
