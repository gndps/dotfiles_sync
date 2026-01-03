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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub encryption_key_path: Option<PathBuf>,
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
            encryption_key_path: None,
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

    /// Resolve repo path from local config (home directory) or use provided path
    pub fn resolve_repo_path() -> Result<PathBuf> {
        // Get the local config path (respecting env variable)
        let local_config_path = if let Ok(env_path) = std::env::var("DOTFILES_LOCAL_CONFIG_FILE") {
            PathBuf::from(env_path)
        } else if let Some(home) = dirs::home_dir() {
            home.join(".dotfiles.config.local.json")
        } else {
            PathBuf::new()
        };
        
        // Check if local config exists
        if local_config_path.exists() {
            let content = fs::read_to_string(&local_config_path)
                .context("Failed to read local config file")?;
            let local: DotfilesConfig = serde_json::from_str(&content)
                .context("Failed to parse local config file")?;
            
            // Return the repo path from local config
            return Ok(local.repo_path);
        }
        
        // Fall back to current directory if no local config exists
        std::env::current_dir().context("Failed to get current directory")
    }

    pub fn get_config_path(&self) -> PathBuf {
        self.repo_path.join(DOTFILES_CONFIG)
    }

    pub fn get_local_config_path(&self) -> PathBuf {
        // Check for environment variable first
        if let Ok(env_path) = std::env::var("DOTFILES_LOCAL_CONFIG_FILE") {
            return PathBuf::from(env_path);
        }
        
        // Fall back to default path in home directory
        if let Some(home) = dirs::home_dir() {
            home.join(".dotfiles.config.local.json")
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
            if local.encryption_key_path.is_some() {
                config.encryption_key_path = local.encryption_key_path;
            }
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

    pub fn save_local_config(&self, repo_path: PathBuf) -> Result<()> {
        let local_config_path = self.get_local_config_path();
        
        let mut local_config = if local_config_path.exists() {
            let content = fs::read_to_string(&local_config_path)
                .context("Failed to read local config file")?;
            serde_json::from_str(&content)
                .context("Failed to parse local config file")?
        } else {
            DotfilesConfig::default()
        };
        
        local_config.repo_path = repo_path;
        
        let content = serde_json::to_string_pretty(&local_config)
            .context("Failed to serialize local config")?;
        
        fs::write(&local_config_path, content)
            .context("Failed to write local config file")?;
        
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
