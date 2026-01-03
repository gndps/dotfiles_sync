use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

pub const DOTFILES_CONFIG: &str = "dotfiles.config.json";
pub const DOTFILES_LOCAL_CONFIG: &str = "dotfiles.local.config.json";

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DotfilesConfig {
    pub use_xdg: bool,
    pub repo_path: PathBuf,
    pub home_path: PathBuf,
    pub tag: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tracked_files: Option<Vec<TrackedFile>>,
}

impl Default for DotfilesConfig {
    fn default() -> Self {
        Self {
            use_xdg: false,
            repo_path: PathBuf::from("."),
            home_path: dirs::home_dir().unwrap_or_else(|| PathBuf::from("~")),
            tag: None,
            tracked_files: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrackedFile {
    pub stub: Option<String>,
    pub path: String,
    pub encrypted: bool,
}

pub struct ConfigManager {
    repo_path: PathBuf,
}

impl ConfigManager {
    pub fn new(repo_path: PathBuf) -> Self {
        Self { repo_path }
    }

    pub fn get_config_path(&self) -> PathBuf {
        self.repo_path.join(DOTFILES_CONFIG)
    }

    pub fn get_local_config_path(&self) -> PathBuf {
        if let Some(home) = dirs::home_dir() {
            home.join(".dotfiles.local.config.json")
        } else {
            self.repo_path.join(DOTFILES_LOCAL_CONFIG)
        }
    }

    pub fn load_config(&self) -> Result<DotfilesConfig> {
        let mut config = DotfilesConfig::default();
        
        // Load main config
        let config_path = self.get_config_path();
        if config_path.exists() {
            let content = fs::read_to_string(&config_path)
                .context("Failed to read config file")?;
            config = serde_json::from_str(&content)
                .context("Failed to parse config file")?;
        }
        
        // Merge with local config (takes precedence)
        let local_config_path = self.get_local_config_path();
        if local_config_path.exists() {
            let content = fs::read_to_string(&local_config_path)
                .context("Failed to read local config file")?;
            let local: DotfilesConfig = serde_json::from_str(&content)
                .context("Failed to parse local config file")?;
            
            // Local overrides main
            config.use_xdg = local.use_xdg;
            config.repo_path = local.repo_path;
            config.home_path = local.home_path;
            if local.tag.is_some() {
                config.tag = local.tag;
            }
        }
        
        Ok(config)
    }

    pub fn save_config(&self, config: &DotfilesConfig) -> Result<()> {
        let config_path = self.get_config_path();
        let content = serde_json::to_string_pretty(config)
            .context("Failed to serialize config")?;
        
        fs::write(&config_path, content)
            .context("Failed to write config file")?;
        
        Ok(())
    }

    pub fn load_tracked_files(&self) -> Result<Vec<TrackedFile>> {
        let config = self.load_config()?;
        
        if let Some(tracked) = config.tracked_files {
            Ok(tracked)
        } else {
            Ok(Vec::new())
        }
    }

    pub fn save_tracked_files(&self, tracked: &[TrackedFile]) -> Result<()> {
        let mut config = self.load_config()?;
        config.tracked_files = Some(tracked.to_vec());
        self.save_config(&config)
    }

    pub fn is_initialized(&self) -> bool {
        self.get_config_path().exists()
    }

    pub fn get_repo_path(&self) -> &Path {
        &self.repo_path
    }
}
