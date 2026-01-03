use anyhow::{bail, Context, Result};
use std::path::Path;
use std::process::Command;

pub struct GitRepo {
    repo_path: Box<Path>,
}

impl GitRepo {
    pub fn new(path: &Path) -> Self {
        Self {
            repo_path: path.into(),
        }
    }

    pub fn is_repo(&self) -> bool {
        self.repo_path.join(".git").exists()
    }

    pub fn init(&self) -> Result<()> {
        self.run_command(&["init"])?;
        Ok(())
    }

    pub fn has_changes(&self) -> Result<bool> {
        let output = Command::new("git")
            .current_dir(&self.repo_path)
            .args(&["status", "--porcelain"])
            .output()
            .context("Failed to check git status")?;

        Ok(!output.stdout.is_empty())
    }

    pub fn is_dirty(&self) -> Result<bool> {
        self.has_changes()
    }

    pub fn is_in_merge(&self) -> Result<bool> {
        let merge_head = self.repo_path.join(".git/MERGE_HEAD");
        Ok(merge_head.exists())
    }

    pub fn is_in_rebase(&self) -> Result<bool> {
        let rebase_merge = self.repo_path.join(".git/rebase-merge");
        let rebase_apply = self.repo_path.join(".git/rebase-apply");
        Ok(rebase_merge.exists() || rebase_apply.exists())
    }

    pub fn has_conflicts(&self) -> Result<bool> {
        let output = Command::new("git")
            .current_dir(&self.repo_path)
            .args(&["diff", "--name-only", "--diff-filter=U"])
            .output()
            .context("Failed to check for conflicts")?;

        Ok(!output.stdout.is_empty())
    }

    pub fn get_conflicted_files(&self) -> Result<Vec<String>> {
        let output = Command::new("git")
            .current_dir(&self.repo_path)
            .args(&["diff", "--name-only", "--diff-filter=U"])
            .output()
            .context("Failed to get conflicted files")?;

        let files = String::from_utf8_lossy(&output.stdout)
            .lines()
            .map(|s| s.to_string())
            .collect();

        Ok(files)
    }

    pub fn rebase_continue(&self) -> Result<()> {
        let output = Command::new("git")
            .current_dir(&self.repo_path)
            .args(&["rebase", "--continue"])
            .output()?;

        if !output.status.success() {
            let error_msg = String::from_utf8_lossy(&output.stderr);
            bail!("Git rebase --continue failed: {}", error_msg);
        }
        Ok(())
    }

    pub fn add_all(&self) -> Result<()> {
        let status = Command::new("git")
            .current_dir(&self.repo_path)
            .args(&["add", "-A"])
            .status()?;
            
        if !status.success() {
            bail!("Failed to add files to git staging");
        }
        Ok(())
    }

    pub fn commit(&self, message: &str) -> Result<()> {
        self.run_command(&["commit", "-m", message])?;
        Ok(())
    }

    pub fn stash(&self, message: &str) -> Result<bool> {
        if !self.has_changes()? {
            return Ok(false);
        }

        self.add_all()?;
        self.run_command(&["stash", "push", "-m", message])?;
        Ok(true)
    }

    pub fn stash_pop(&self) -> Result<bool> {
        let list = self.get_stash_list()?;
        
        if list.is_empty() {
            return Ok(true);
        }

        let status = Command::new("git")
            .current_dir(&self.repo_path)
            .args(&["stash", "pop"])
            .status()
            .context("Failed to pop stash")?;

        Ok(status.success())
    }

    pub fn get_stash_list(&self) -> Result<Vec<String>> {
        let output = Command::new("git")
            .current_dir(&self.repo_path)
            .args(&["stash", "list"])
            .output()
            .context("Failed to list stashes")?;

        let list = String::from_utf8_lossy(&output.stdout)
            .lines()
            .map(|s| s.to_string())
            .collect();

        Ok(list)
    }

    pub fn pull(&self, remote: &str, branch: &str) -> Result<()> {
        self.run_command(&["pull", remote, branch])?;
        Ok(())
    }

    pub fn pull_rebase(&self, remote: &str, branch: &str) -> Result<()> {
        let output = Command::new("git")
            .current_dir(&self.repo_path)
            .args(&["pull", "--rebase", remote, branch])
            .output()?;

        if !output.status.success() {
            let error_msg = String::from_utf8_lossy(&output.stderr);
            bail!("Git rebase failed: {}", error_msg);
        }
        Ok(())
    }

    pub fn push(&self, remote: &str, branch: &str) -> Result<()> {
        self.run_command(&["push", remote, branch])?;
        Ok(())
    }

    pub fn push_set_upstream(&self, remote: &str, branch: &str) -> Result<()> {
        self.run_command(&["push", "-u", remote, branch])?;
        Ok(())
    }

    pub fn has_remote(&self) -> Result<bool> {
        let output = Command::new("git")
            .current_dir(&self.repo_path)
            .args(&["remote"])
            .output()
            .context("Failed to check remotes")?;

        Ok(!output.stdout.is_empty())
    }

    pub fn remote_has_commits(&self, remote: &str, branch: &str) -> Result<bool> {
        // Check if remote has the branch by doing ls-remote
        let output = Command::new("git")
            .current_dir(&self.repo_path)
            .args(&["ls-remote", "--heads", remote, branch])
            .output()
            .context("Failed to check remote refs")?;

        // If output is empty, remote doesn't have this branch
        Ok(!output.stdout.is_empty())
    }

    pub fn get_current_branch(&self) -> Result<String> {
        let output = Command::new("git")
            .current_dir(&self.repo_path)
            .args(&["branch", "--show-current"])
            .output()
            .context("Failed to get current branch")?;

        let branch = String::from_utf8_lossy(&output.stdout).trim().to_string();
        
        if branch.is_empty() {
            // In detached HEAD or old git, try to get branch from symbolic-ref
            let output = Command::new("git")
                .current_dir(&self.repo_path)
                .args(&["symbolic-ref", "--short", "HEAD"])
                .output()
                .context("Failed to get symbolic ref")?;
            
            let symbolic_branch = String::from_utf8_lossy(&output.stdout).trim().to_string();
            
            if !symbolic_branch.is_empty() {
                Ok(symbolic_branch)
            } else {
                bail!("Unable to determine current branch. Make sure you're on a branch, not in detached HEAD state.")
            }
        } else {
            Ok(branch)
        }
    }

    fn run_command(&self, args: &[&str]) -> Result<()> {
        let status = Command::new("git")
            .current_dir(&self.repo_path)
            .args(args)
            .status()
            .context(format!("Failed to execute git {:?}", args))?;

        if !status.success() {
            bail!("Git command failed: {:?}", args);
        }

        Ok(())
    }
}
