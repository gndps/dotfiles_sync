use anyhow::{bail, Result};
use colored::Colorize;
use crate::config::ConfigManager;
use crate::git::GitRepo;
use crate::utils::{print_error, print_info, print_success, print_warning};

pub fn execute() -> Result<()> {
    let repo_path = ConfigManager::resolve_repo_path()?;
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
        print_error("Cannot push: repository has unresolved merge conflicts!");
        println!("\n{}", "Resolution steps:".bold());
        println!("  1. Resolve all conflicts");
        println!("  2. Run: git add <resolved-files>");
        println!("  3. Run: git commit");
        println!("  4. Run: dotfiles push");
        bail!("Merge conflict - cannot push");
    }

    if !git.has_remote()? {
        print_warning("No remote repository configured");
        print_info("Add a remote with: git remote add origin <url>");
        return Ok(());
    }

    print_info("Pushing to remote repository...");

    let branch = git.get_current_branch()?;
    git.push("origin", &branch)?;

    print_success("Push completed successfully");

    Ok(())
}
