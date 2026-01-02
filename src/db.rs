use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

static DEFAULT_DB: OnceLock<HashMap<String, DefaultStubData>> = OnceLock::new();

const DEFAULT_DB_JSON: &str = include_str!("default_db.json");

#[derive(Debug, Clone, Serialize, Deserialize)]
struct DefaultStubData {
    name: String,
    config_files: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct StubEntry {
    pub name: String,
    pub stub: String,
    pub config_files: Vec<String>,
    pub is_custom: bool,
}

pub struct ConfigDatabase {
    custom_db_path: PathBuf,
}

impl ConfigDatabase {
    pub fn new(repo_path: &Path) -> Self {
        Self::init_default_db();
        Self {
            custom_db_path: repo_path.join("custom_db"),
        }
    }

    pub fn new_with_tag(repo_path: &Path, tag: Option<&str>) -> Self {
        Self::init_default_db();
        let custom_path = if let Some(t) = tag {
            repo_path.join("custom_db").join(t)
        } else {
            repo_path.join("custom_db")
        };
        
        Self {
            custom_db_path: custom_path,
        }
    }

    fn init_default_db() {
        DEFAULT_DB.get_or_init(|| {
            serde_json::from_str(DEFAULT_DB_JSON)
                .expect("Failed to parse embedded default database")
        });
    }

    fn get_default_db() -> &'static HashMap<String, DefaultStubData> {
        DEFAULT_DB.get().expect("Default DB not initialized")
    }

    pub fn load_stub(&self, stub: &str) -> Result<Option<StubEntry>> {
        if let Some(entry) = self.load_stub_from_path(&self.custom_db_path, stub, true)? {
            return Ok(Some(entry));
        }
        
        self.load_stub_from_embedded(stub)
    }

    fn load_stub_from_embedded(&self, stub: &str) -> Result<Option<StubEntry>> {
        let db = Self::get_default_db();
        
        if let Some(data) = db.get(stub) {
            Ok(Some(StubEntry {
                name: data.name.clone(),
                stub: stub.to_string(),
                config_files: data.config_files.clone(),
                is_custom: false,
            }))
        } else {
            Ok(None)
        }
    }

    fn load_stub_from_path(&self, base_path: &Path, stub: &str, is_custom: bool) -> Result<Option<StubEntry>> {
        let applications_path = base_path.join("applications").join(format!("{}.conf", stub));
        
        if !applications_path.exists() {
            return Ok(None);
        }

        let name = fs::read_to_string(&applications_path)
            .context("Failed to read application name")?
            .lines()
            .find(|line| line.starts_with("name = "))
            .and_then(|line| line.strip_prefix("name = "))
            .unwrap_or(stub)
            .trim()
            .to_string();

        let config_files = self.read_file_list(&base_path.join("default_configs").join(format!("{}.conf", stub)))?;

        Ok(Some(StubEntry {
            name,
            stub: stub.to_string(),
            config_files,
            is_custom,
        }))
    }

    pub fn list_all_stubs(&self) -> Result<Vec<String>> {
        let mut stubs = std::collections::HashSet::new();
        
        // Add default stubs from embedded JSON
        let db = Self::get_default_db();
        for stub in db.keys() {
            stubs.insert(stub.clone());
        }
        
        // Add custom stubs from filesystem
        let apps_dir = self.custom_db_path.join("applications");
        if apps_dir.exists() {
            for entry in fs::read_dir(&apps_dir)? {
                let entry = entry?;
                let path = entry.path();
                
                if path.extension().and_then(|s| s.to_str()) == Some("conf") {
                    if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                        stubs.insert(stem.to_string());
                    }
                }
            }
        }
        
        let mut result: Vec<_> = stubs.into_iter().collect();
        result.sort();
        Ok(result)
    }

    pub fn create_stub(&self, stub: &str, name: &str, paths: &[String]) -> Result<()> {
        fs::create_dir_all(self.custom_db_path.join("applications"))?;
        fs::create_dir_all(self.custom_db_path.join("default_configs"))?;

        let app_path = self.custom_db_path.join("applications").join(format!("{}.conf", stub));
        fs::write(&app_path, format!("name = {}\n", name))?;

        let config_path = self.custom_db_path.join("default_configs").join(format!("{}.conf", stub));
        let content = paths.join("\n") + "\n";
        fs::write(&config_path, content)?;

        Ok(())
    }

    fn read_file_list(&self, path: &Path) -> Result<Vec<String>> {
        if !path.exists() {
            return Ok(Vec::new());
        }

        let content = fs::read_to_string(path)?;
        Ok(content
            .lines()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect())
    }

    pub fn get_stub_info(&self, stub: &str) -> Result<Option<(String, Vec<String>, bool)>> {
        let entry = self.load_stub(stub)?;
        
        Ok(entry.map(|e| {
            (e.name, e.config_files.clone(), e.is_custom)
        }))
    }

    pub fn get_default_stubs(&self) -> Result<HashMap<String, StubEntry>> {
        let db = Self::get_default_db();
        let mut result = HashMap::new();
        
        for (stub_name, data) in db.iter() {
            result.insert(
                stub_name.clone(),
                StubEntry {
                    name: data.name.clone(),
                    stub: stub_name.clone(),
                    config_files: data.config_files.clone(),
                    is_custom: false,
                }
            );
        }
        
        Ok(result)
    }

    pub fn get_custom_stubs(&self) -> Result<HashMap<String, StubEntry>> {
        let mut result = HashMap::new();
        
        let apps_dir = self.custom_db_path.join("applications");
        if !apps_dir.exists() {
            return Ok(result);
        }
        
        for entry in fs::read_dir(&apps_dir)? {
            let entry = entry?;
            let path = entry.path();
            
            if path.extension().and_then(|s| s.to_str()) == Some("conf") {
                if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                    if let Some(stub_entry) = self.load_stub_from_path(&self.custom_db_path, stem, true)? {
                        result.insert(stem.to_string(), stub_entry);
                    }
                }
            }
        }
        
        Ok(result)
    }
}
