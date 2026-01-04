use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

pub const DOTFILES_CONFIG: &str = "dotfiles.config.json";
pub const DOTFILES_LOCAL_CONFIG: &str = ".dotfiles.local.config.json";
pub const ENV_LOCAL_CONFIG: &str = "DOTFILES_LOCAL_CONFIG_FILEPATH";

// Repository config - stored in repo (dotfiles.config.json)
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DotfilesConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tracked_files: Option<Vec<TrackedFile>>,
}

// Local config - stored in home directory (~/.dotfiles.local.config.json)
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LocalConfig {
    pub use_xdg: bool,
    pub repo_path: PathBuf,
    pub home_path: PathBuf,
    pub tag: Option<String>,
}

impl Default for DotfilesConfig {
    fn default() -> Self {
        Self {
            tracked_files: None,
        }
    }
}

impl Default for LocalConfig {
    fn default() -> Self {
        Self {
            use_xdg: false,
            repo_path: PathBuf::from("."),
            home_path: dirs::home_dir().unwrap_or_else(|| PathBuf::from("~")),
            tag: None,
        }
    }
}

// Combined config for runtime use
#[derive(Debug, Clone)]
pub struct RuntimeConfig {
    pub use_xdg: bool,
    pub repo_path: PathBuf,
    pub home_path: PathBuf,
    pub tag: Option<String>,
    pub tracked_files: Vec<TrackedFile>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrackedFile {
    pub stub: Option<String>,
    pub path: String,
}

pub struct ConfigManager {
    repo_path: PathBuf,
}

impl ConfigManager {
    pub fn new(repo_path: PathBuf) -> Self {
        Self { repo_path }
    }

    /// Get the path to local config file (checks env var first)
    pub fn get_local_config_file_path() -> PathBuf {
        // Check environment variable first
        if let Ok(env_path) = std::env::var(ENV_LOCAL_CONFIG) {
            return PathBuf::from(env_path);
        }
        
        // Default to home directory
        if let Some(home) = dirs::home_dir() {
            home.join(DOTFILES_LOCAL_CONFIG)
        } else {
            PathBuf::from(DOTFILES_LOCAL_CONFIG)
        }
    }

    /// Resolve repo path from local config (env var or home directory)
    pub fn resolve_repo_path() -> Result<PathBuf> {
        let local_config_path = Self::get_local_config_file_path();
        
        if local_config_path.exists() {
            let content = fs::read_to_string(&local_config_path)
                .context("Failed to read local config file")?;
            let local: LocalConfig = serde_json::from_str(&content)
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
        Self::get_local_config_file_path()
    }

    pub fn load_config(&self) -> Result<DotfilesConfig> {
        let config_path = self.get_config_path();
        if config_path.exists() {
            let content = fs::read_to_string(&config_path)
                .context("Failed to read config file")?;
            serde_json::from_str(&content)
                .context("Failed to parse config file")
        } else {
            Ok(DotfilesConfig::default())
        }
    }

    pub fn load_local_config(&self) -> Result<LocalConfig> {
        let local_config_path = self.get_local_config_path();
        if local_config_path.exists() {
            let content = fs::read_to_string(&local_config_path)
                .context("Failed to read local config file")?;
            serde_json::from_str(&content)
                .context("Failed to parse local config file")
        } else {
            Ok(LocalConfig::default())
        }
    }

    pub fn load_runtime_config(&self) -> Result<RuntimeConfig> {
        let repo_config = self.load_config()?;
        let local_config = self.load_local_config()?;
        
        Ok(RuntimeConfig {
            use_xdg: local_config.use_xdg,
            repo_path: local_config.repo_path,
            home_path: local_config.home_path,
            tag: local_config.tag,
            tracked_files: repo_config.tracked_files.unwrap_or_default(),
        })
    }

    pub fn save_config(&self, config: &DotfilesConfig) -> Result<()> {
        let config_path = self.get_config_path();
        let content = serde_json::to_string_pretty(config)
            .context("Failed to serialize config")?;
        
        fs::write(&config_path, content)
            .context("Failed to write config file")?;
        
        Ok(())
    }

    pub fn save_local_config(&self, local_config: &LocalConfig) -> Result<()> {
        let local_config_path = self.get_local_config_path();
        
        let content = serde_json::to_string_pretty(local_config)
            .context("Failed to serialize local config")?;
        
        fs::write(&local_config_path, content)
            .context("Failed to write local config file")?;
        
        Ok(())
    }

    pub fn update_local_config_field(&self, field: &str, value: &str) -> Result<()> {
        let mut local_config = self.load_local_config()?;
        
        match field {
            "use_xdg" => {
                local_config.use_xdg = value.parse::<bool>()
                    .context("Invalid boolean value for use_xdg")?;
            }
            "repo_path" => {
                local_config.repo_path = PathBuf::from(value);
            }
            "home_path" => {
                local_config.home_path = PathBuf::from(value);
            }
            "tag" => {
                local_config.tag = if value.is_empty() {
                    None
                } else {
                    Some(value.to_string())
                };
            }
            _ => anyhow::bail!("Unknown config field: {}", field),
        }
        
        self.save_local_config(&local_config)
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
