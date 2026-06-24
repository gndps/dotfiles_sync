use anyhow::{bail, Result};
use crate::config::ConfigManager;
use std::path::{Path, PathBuf};
use std::process::Command;

pub fn execute(path: Option<String>) -> Result<()> {
    let repo_path = ConfigManager::resolve_repo_path()?;
    let manager = ConfigManager::new(repo_path.clone());

    if !manager.is_initialized() {
        bail!("Not in a dotfiles repository. Run 'dotfiles init' first.");
    }

    let local_config = manager.load_local_config()?;
    let editor = &local_config.editor;

    let open_path: PathBuf = match path {
        Some(p) => resolve_to_repo_copy(&p, &repo_path),
        None => {
            // Try fzf picker over tracked files; fall back to opening the repo folder
            let tracked = manager.load_tracked_files()?;
            if !tracked.is_empty() && is_fzf_available() {
                let choices: Vec<String> = tracked
                    .iter()
                    .map(|f| f.path.trim_start_matches("~/").to_string())
                    .collect();
                let input = choices.join("\n");
                let selected = run_fzf(&input, "📄 Managed file> ")?;
                if selected.is_empty() {
                    return Ok(());
                }
                repo_path.join(&selected)
            } else {
                repo_path.clone()
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

/// Resolve user-provided path to the repo copy (never the home directory version).
/// - Relative path  → joined with repo_path
/// - Absolute path inside repo_path → used as-is
/// - home-relative (~/) → strip ~/ and join with repo_path
/// - Any other absolute → strip home prefix if present, then join with repo_path
fn resolve_to_repo_copy(input: &str, repo_path: &Path) -> PathBuf {
    // Tilde-relative: strip ~/ and join with repo
    if let Some(rel) = input.strip_prefix("~/") {
        return repo_path.join(rel);
    }

    let p = Path::new(input);

    if !p.is_absolute() {
        return repo_path.join(p);
    }

    // Already inside the repo copy
    if p.starts_with(repo_path) {
        return p.to_path_buf();
    }

    // Strip home dir prefix if present
    if let Some(home) = dirs::home_dir() {
        if let Ok(rel) = p.strip_prefix(&home) {
            return repo_path.join(rel);
        }
    }

    // Fallback: use as-is
    p.to_path_buf()
}

fn is_fzf_available() -> bool {
    Command::new("fzf")
        .arg("--version")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

fn run_fzf(input: &str, prompt: &str) -> Result<String> {
    use std::io::Write;
    let mut child = Command::new("fzf")
        .args(["--no-sort", "--prompt", prompt, "--height=~40%", "--border"])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| anyhow::anyhow!("Failed to launch fzf: {}", e))?;

    if let Some(stdin) = child.stdin.as_mut() {
        let _ = stdin.write_all(input.as_bytes());
    }

    let out = child.wait_with_output()?;
    Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
}
