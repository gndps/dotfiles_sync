use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

pub struct FileSyncer;

impl FileSyncer {
    pub fn sync_file(source: &Path, dest: &Path) -> Result<()> {
        if let Some(parent) = dest.parent() {
            fs::create_dir_all(parent)
                .context(format!("Failed to create parent directory: {:?}", parent))?;
        }

        if source.is_dir() {
            Self::sync_directory(source, dest)?;
        } else {
            fs::copy(source, dest)
                .context(format!("Failed to copy {:?} to {:?}", source, dest))?;
        }

        Ok(())
    }

    pub fn sync_directory(source: &Path, dest: &Path) -> Result<()> {
        fs::create_dir_all(dest)
            .context(format!("Failed to create directory: {:?}", dest))?;

        for entry in WalkDir::new(source).min_depth(1) {
            let entry = entry?;
            let relative_path = entry.path().strip_prefix(source)?;
            let dest_path = dest.join(relative_path);

            if entry.file_type().is_dir() {
                fs::create_dir_all(&dest_path)?;
            } else {
                if let Some(parent) = dest_path.parent() {
                    fs::create_dir_all(parent)?;
                }
                fs::copy(entry.path(), &dest_path)?;
            }
        }

        Ok(())
    }

    pub fn expand_tilde(path: &str) -> PathBuf {
        if path.starts_with("~/") {
            if let Some(home) = dirs::home_dir() {
                return home.join(&path[2..]);
            }
        } else if path.starts_with('/') {
            // Absolute path, use as-is
            return PathBuf::from(path);
        } else {
            // Relative path without ~/ prefix (e.g., from mackup default_db)
            // Assume it's relative to home directory
            if let Some(home) = dirs::home_dir() {
                return home.join(path);
            }
        }
        PathBuf::from(path)
    }

    pub fn strip_tilde(path: &Path) -> Option<String> {
        if let Some(home) = dirs::home_dir() {
            if let Ok(relative) = path.strip_prefix(&home) {
                return Some(format!("~/{}", relative.display()));
            }
        }
        None
    }
}

pub fn get_relative_repo_path(home_path: &str) -> String {
    let path = home_path.trim_start_matches("~/");
    path.to_string()
}
