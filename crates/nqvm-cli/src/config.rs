use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

pub const DEFAULT_API_URL: &str = "http://127.0.0.1:18080";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Config {
    #[serde(default = "default_api_url")]
    pub api_url: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub token: Option<String>,
    #[serde(default = "default_output")]
    pub default_output: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            api_url: default_api_url(),
            token: None,
            default_output: default_output(),
        }
    }
}

pub fn default_api_url() -> String {
    DEFAULT_API_URL.to_string()
}

fn default_output() -> String {
    "table".to_string()
}

pub fn default_config_path() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("nqvm")
        .join("config.toml")
}

impl Config {
    pub fn load(path: &Path) -> Result<Self> {
        if !path.exists() {
            return Ok(Self::default());
        }

        let raw = std::fs::read_to_string(path)
            .with_context(|| format!("reading config {}", path.display()))?;
        toml::from_str(&raw).with_context(|| format!("parsing config {}", path.display()))
    }

    pub fn save(&self, path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("creating config dir {}", parent.display()))?;
        }

        let raw = toml::to_string_pretty(self).context("serializing config")?;
        std::fs::write(path, raw).with_context(|| format!("writing config {}", path.display()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_config_uses_defaults() {
        let dir = tempfile::tempdir().unwrap();
        let cfg = Config::load(&dir.path().join("missing.toml")).unwrap();
        assert_eq!(cfg.api_url, DEFAULT_API_URL);
        assert_eq!(cfg.default_output, "table");
        assert_eq!(cfg.token, None);
    }

    #[test]
    fn save_and_load_config() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("nqvm/config.toml");
        let cfg = Config {
            api_url: "http://manager:18080".into(),
            token: Some("abc".into()),
            default_output: "json".into(),
        };

        cfg.save(&path).unwrap();
        assert_eq!(Config::load(&path).unwrap(), cfg);
    }
}
