use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use walkdir::WalkDir;
use crate::utils::{print_info, print_success};

const MACKUP_REPO: &str = "https://github.com/lra/mackup.git";

pub struct MackupSync {
    temp_dir: PathBuf,
}

impl MackupSync {
    pub fn new() -> Result<Self> {
        let temp_dir = std::env::temp_dir().join("dotfiles_mackup_sync");
        Ok(Self { temp_dir })
    }

    pub fn sync_to_config_db(&self, output_dir: &Path) -> Result<()> {
        print_info("Syncing configuration database from mackup repository...");
        
        self.clone_mackup()?;
        
        let apps_source = self.find_applications_dir()?;
        let cfg_files: Vec<_> = WalkDir::new(&apps_source)
            .max_depth(1)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| {
                e.path()
                    .extension()
                    .and_then(|s| s.to_str())
                    .map(|s| s == "cfg")
                    .unwrap_or(false)
            })
            .collect();

        print_info(&format!("Found {} application configurations", cfg_files.len()));

        let mut processed = 0;
        let mut skipped = 0;

        for entry in cfg_files {
            let cfg_path = entry.path();
            let stub_name = cfg_path.file_stem().and_then(|s| s.to_str());

            if let Some(stub) = stub_name {
                match self.process_cfg_file(cfg_path, stub, output_dir) {
                    Ok(true) => processed += 1,
                    Ok(false) => skipped += 1,
                    Err(_) => skipped += 1,
                }
            }
        }

        self.cleanup()?;

        print_success(&format!(
            "Processed {} applications, skipped {}",
            processed, skipped
        ));

        Ok(())
    }

    fn clone_mackup(&self) -> Result<()> {
        if self.temp_dir.exists() {
            print_info("Removing existing temporary directory...");
            fs::remove_dir_all(&self.temp_dir)
                .context("Failed to remove existing temp directory")?;
        }

        print_info("Cloning mackup repository (this may take a moment)...");

        let status = Command::new("git")
            .args(&[
                "clone",
                "--depth=1",
                "--single-branch",
                MACKUP_REPO,
                self.temp_dir.to_str().unwrap(),
            ])
            .status()
            .context("Failed to execute git clone")?;

        if !status.success() {
            anyhow::bail!("Failed to clone mackup repository");
        }

        print_success("Mackup repository cloned");
        Ok(())
    }

    fn find_applications_dir(&self) -> Result<PathBuf> {
        let possible_paths = vec![
            self.temp_dir.join("mackup").join("applications"),
            self.temp_dir.join("src").join("mackup").join("applications"),
        ];

        for path in possible_paths {
            if path.exists() && path.is_dir() {
                return Ok(path);
            }
        }

        anyhow::bail!(
            "Could not find applications directory in mackup repository at {:?}",
            self.temp_dir
        )
    }

    fn process_cfg_file(
        &self,
        cfg_path: &Path,
        stub_name: &str,
        output_dir: &Path,
    ) -> Result<bool> {
        let content = fs::read_to_string(cfg_path)
            .context("Failed to read .cfg file")?;

        let (name, config_files, xdg_files) = Self::parse_cfg_content(&content)?;

        if name.is_empty() && config_files.is_empty() && xdg_files.is_empty() {
            return Ok(false);
        }

        self.create_flat_structure(stub_name, &name, &config_files, &xdg_files, output_dir)?;

        Ok(true)
    }

    fn parse_cfg_content(content: &str) -> Result<(String, Vec<String>, Vec<String>)> {
        let mut name = String::new();
        let mut config_files = Vec::new();
        let mut xdg_files = Vec::new();
        let mut current_section = String::new();

        for line in content.lines() {
            let line = line.trim();
            
            if line.is_empty() || line.starts_with('#') || line.starts_with(';') {
                continue;
            }

            if line.starts_with('[') && line.ends_with(']') {
                current_section = line[1..line.len()-1].to_string();
                continue;
            }

            if current_section == "application" {
                if let Some((key, value)) = line.split_once('=') {
                    let key = key.trim();
                    let value = value.trim();
                    if key == "name" {
                        name = value.to_string();
                    }
                }
            } else if current_section == "configuration_files" {
                if !line.is_empty() && !line.contains('=') {
                    config_files.push(line.to_string());
                }
            } else if current_section == "xdg_configuration_files" {
                if !line.is_empty() && !line.contains('=') {
                    xdg_files.push(line.to_string());
                }
            }
        }

        Ok((name, config_files, xdg_files))
    }

    fn create_flat_structure(
        &self,
        stub_name: &str,
        app_name: &str,
        config_files: &[String],
        xdg_files: &[String],
        output_dir: &Path,
    ) -> Result<()> {
        let apps_dir = output_dir.join("applications");
        let config_files_dir = output_dir.join("configuration_files");
        let xdg_files_dir = output_dir.join("xdg_configuration_files");

        fs::create_dir_all(&apps_dir)?;
        fs::create_dir_all(&config_files_dir)?;
        fs::create_dir_all(&xdg_files_dir)?;

        if !app_name.is_empty() {
            let app_file = apps_dir.join(format!("{}.conf", stub_name));
            fs::write(&app_file, format!("name = {}\n", app_name))?;
        }

        let config_file = config_files_dir.join(format!("{}.conf", stub_name));
        if !config_files.is_empty() {
            let content = config_files.join("\n") + "\n";
            fs::write(&config_file, content)?;
        } else {
            fs::write(&config_file, "")?;
        }

        let xdg_file = xdg_files_dir.join(format!("{}.conf", stub_name));
        if !xdg_files.is_empty() {
            let content = xdg_files.join("\n") + "\n";
            fs::write(&xdg_file, content)?;
        } else {
            fs::write(&xdg_file, "")?;
        }

        Ok(())
    }

    fn cleanup(&self) -> Result<()> {
        if self.temp_dir.exists() {
            fs::remove_dir_all(&self.temp_dir).context("Failed to cleanup temp directory")?;
        }
        Ok(())
    }
}

impl Drop for MackupSync {
    fn drop(&mut self) {
        let _ = self.cleanup();
    }
}
