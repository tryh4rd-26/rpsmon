use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Config {
    #[serde(default)]
    pub theme: String,
    #[serde(default)]
    pub sort_by: String,
    #[serde(default)]
    pub refresh_rate: u64,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            theme: "nord".to_string(),
            sort_by: "cpu".to_string(),
            refresh_rate: 16,
        }
    }
}

impl Config {
    pub fn config_dir() -> Result<PathBuf> {
        let dir = match dirs::config_dir() {
            Some(d) => d.join("rps"),
            None => return Err(anyhow::anyhow!("Could not determine config directory")),
        };

        if !dir.exists() {
            std::fs::create_dir_all(&dir)?;
        }

        Ok(dir)
    }

    pub fn config_path() -> Result<PathBuf> {
        Ok(Self::config_dir()?.join("config.toml"))
    }

    pub fn load() -> Result<Self> {
        let path = Self::config_path()?;

        if path.exists() {
            let content = std::fs::read_to_string(&path)?;
            Ok(toml::from_str(&content)?)
        } else {
            Ok(Self::default())
        }
    }

    pub fn save(&self) -> Result<()> {
        let path = Self::config_path()?;
        let content = toml::to_string_pretty(self)?;
        std::fs::write(path, content)?;
        Ok(())
    }
}
