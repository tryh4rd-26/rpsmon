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
    #[serde(default)]
    pub watchlist_pids: Vec<u32>,
    #[serde(default = "default_alerts_enabled")]
    pub alerts_enabled: bool,
    #[serde(default = "default_alert_cpu_pct")]
    pub alert_cpu_pct: f32,
    #[serde(default = "default_alert_mem_pct")]
    pub alert_mem_pct: f32,
    #[serde(default = "default_alert_hold_secs")]
    pub alert_hold_secs: u64,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            theme: "nord".to_string(),
            sort_by: "cpu".to_string(),
            refresh_rate: 16,
            watchlist_pids: Vec::new(),
            alerts_enabled: true,
            alert_cpu_pct: default_alert_cpu_pct(),
            alert_mem_pct: default_alert_mem_pct(),
            alert_hold_secs: default_alert_hold_secs(),
        }
    }
}

impl Config {
    pub fn config_dir() -> Result<PathBuf> {
        let dir = match dirs::config_dir() {
            Some(d) => d.join("rpsmon"),
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
            // Backward-compatible fallback for previous path (~/.config/rps/config.toml).
            if let Some(base) = dirs::config_dir() {
                let legacy = base.join("rps").join("config.toml");
                if legacy.exists() {
                    let content = std::fs::read_to_string(legacy)?;
                    return Ok(toml::from_str(&content)?);
                }
            }
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

fn default_alerts_enabled() -> bool {
    true
}

fn default_alert_cpu_pct() -> f32 {
    80.0
}

fn default_alert_mem_pct() -> f32 {
    20.0
}

fn default_alert_hold_secs() -> u64 {
    5
}
