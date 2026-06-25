use anyhow::{bail, Result};
use crate::config::{ConfigManager, TrackedFile};
use dialoguer::FuzzySelect;
use std::path::{Path, PathBuf};
use std::process::Command;

pub fn execute(input: Option<String>) -> Result<()> {
    let repo_path = ConfigManager::resolve_repo_path()?;
    let manager = ConfigManager::new(repo_path.clone());

    if !manager.is_initialized() {
        bail!("Not in a dotfiles repository. Run 'dotfiles init' first.");
    }

    let local_config = manager.load_local_config()?;
    let editor = &local_config.editor;
    let tracked = manager.load_tracked_files()?;

    let open_path: PathBuf = match input {
        Some(ref s) => resolve_input(s, &tracked, &repo_path)?,
        None => {
            if tracked.is_empty() {
                repo_path.clone()
            } else {
                pick_from_tracked(&tracked, &repo_path)?
            }
        }
    };

    eprintln!("Opening {} in {}", open_path.display(), editor);
    Command::new(editor)
        .arg(&open_path)
        .status()
        .map_err(|e| anyhow::anyhow!("Failed to open editor '{}': {}", editor, e))?;

    Ok(())
}

/// Decide whether input is a stub name or a path, then resolve to the repo copy.
fn resolve_input(input: &str, tracked: &[TrackedFile], repo_path: &Path) -> Result<PathBuf> {
    // Treat as a path if it contains '/' or starts with '~'
    let looks_like_path = input.contains('/') || input.starts_with('~');

    if !looks_like_path {
        // Try stub lookup first
        let stub_files: Vec<&TrackedFile> = tracked
            .iter()
            .filter(|f| f.stub.as_deref() == Some(input))
            .collect();

        if !stub_files.is_empty() {
            return resolve_stub_files(&stub_files, repo_path);
        }
    }

    // Path: resolve to the repo copy
    Ok(resolve_to_repo_copy(input, repo_path))
}

/// One file → open it directly. Multiple files → fuzzy picker.
fn resolve_stub_files(files: &[&TrackedFile], repo_path: &Path) -> Result<PathBuf> {
    if files.len() == 1 {
        let rel = files[0].path.trim_start_matches("~/");
        return Ok(repo_path.join(rel));
    }

    let labels: Vec<String> = files
        .iter()
        .map(|f| f.path.trim_start_matches("~/").to_string())
        .collect();

    let idx = FuzzySelect::new()
        .with_prompt("Select file to edit")
        .items(&labels)
        .interact_opt()?
        .ok_or_else(|| anyhow::anyhow!("Cancelled"))?;

    Ok(repo_path.join(&labels[idx]))
}

/// All tracked files → fuzzy picker. Falls back to repo folder if empty.
fn pick_from_tracked(tracked: &[TrackedFile], repo_path: &Path) -> Result<PathBuf> {
    let labels: Vec<String> = tracked
        .iter()
        .map(|f| {
            let rel = f.path.trim_start_matches("~/");
            match &f.stub {
                Some(s) => format!("{:<20} {}", s, rel),
                None => rel.to_string(),
            }
        })
        .collect();

    let idx = FuzzySelect::new()
        .with_prompt("📄 Managed file")
        .items(&labels)
        .interact_opt()?
        .ok_or_else(|| anyhow::anyhow!("Cancelled"))?;

    let rel = tracked[idx].path.trim_start_matches("~/");
    Ok(repo_path.join(rel))
}

/// Resolve a path argument to the repo copy (never the home directory version).
/// - `~/foo`     → repo_path/foo
/// - `./foo`     → repo_path/foo (relative)
/// - absolute inside repo_path → as-is
/// - absolute inside home dir  → swap to repo copy
/// - folder     → opened as-is (editor handles directories)
fn resolve_to_repo_copy(input: &str, repo_path: &Path) -> PathBuf {
    if let Some(rel) = input.strip_prefix("~/") {
        return repo_path.join(rel);
    }

    let p = Path::new(input);

    if !p.is_absolute() {
        return repo_path.join(p);
    }

    if p.starts_with(repo_path) {
        return p.to_path_buf();
    }

    if let Some(home) = dirs::home_dir() {
        if let Ok(rel) = p.strip_prefix(&home) {
            return repo_path.join(rel);
        }
    }

    p.to_path_buf()
}
