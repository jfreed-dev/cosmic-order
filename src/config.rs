// SPDX-License-Identifier: GPL-3.0-only

//! Application configuration management
//!
//! Uses cosmic-config for persistent storage.

use cosmic_config::CosmicConfigEntry;
use cosmic_config::cosmic_config_derive::CosmicConfigEntry;
use serde::{Deserialize, Serialize};

use crate::pages::PageId;

/// Application identifier for cosmic-config (must match `crate::APP_ID`)
const APP_ID: &str = crate::APP_ID;

/// Application configuration
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, CosmicConfigEntry, PartialEq)]
#[version = 1]
pub struct Config {
    /// Last active page
    pub active_page: PageId,
    /// Window width
    pub window_width: u32,
    /// Window height
    pub window_height: u32,
}

impl Config {
    /// Load configuration from cosmic-config
    pub fn load() -> Result<Self, ConfigError> {
        let config = cosmic_config::Config::new(APP_ID, Self::VERSION)
            .map_err(|e| ConfigError::Load(e.to_string()))?;

        match Self::get_entry(&config) {
            Ok(cfg) => Ok(cfg),
            Err((errors, cfg)) => {
                // Log real errors, ignore missing config (expected on first run)
                for err in errors {
                    if err.is_err() {
                        tracing::warn!("Config load warning: {err}");
                    }
                }
                Ok(cfg)
            }
        }
    }

    /// Save configuration to cosmic-config
    pub fn save(&self) -> Result<(), ConfigError> {
        let config = cosmic_config::Config::new(APP_ID, Self::VERSION)
            .map_err(|e| ConfigError::Save(e.to_string()))?;

        self.write_entry(&config)
            .map_err(|e| ConfigError::Save(e.to_string()))
    }
}

/// Configuration errors
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("Failed to load configuration: {0}")]
    Load(String),
    #[error("Failed to save configuration: {0}")]
    Save(String),
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn test_config_default() {
        let config = Config::default();
        assert_eq!(config.active_page, PageId::Visuals);
        assert_eq!(config.window_width, 0);
        assert_eq!(config.window_height, 0);
    }

    #[test]
    fn test_config_clone() {
        let config = Config {
            active_page: PageId::ToolSync,
            window_width: 1024,
            window_height: 768,
        };
        let cloned = config;
        assert_eq!(cloned.active_page, PageId::ToolSync);
        assert_eq!(cloned.window_width, 1024);
        assert_eq!(cloned.window_height, 768);
    }

    #[test]
    fn test_config_equality() {
        let a = Config {
            active_page: PageId::Screensaver,
            window_width: 800,
            window_height: 600,
        };
        let b = Config {
            active_page: PageId::Screensaver,
            window_width: 800,
            window_height: 600,
        };
        assert_eq!(a, b);
    }

    #[test]
    fn test_config_inequality() {
        let a = Config::default();
        let b = Config {
            active_page: PageId::ToolSync,
            ..Config::default()
        };
        assert_ne!(a, b);
    }

    #[test]
    fn test_config_serialization_roundtrip() {
        let config = Config {
            active_page: PageId::ToolSync,
            window_width: 1920,
            window_height: 1080,
        };
        let json = serde_json::to_string(&config).unwrap();
        let deserialized: Config = serde_json::from_str(&json).unwrap();
        assert_eq!(config, deserialized);
    }

    #[test]
    fn test_config_error_display() {
        let load_err = ConfigError::Load("connection refused".to_string());
        assert!(load_err.to_string().contains("connection refused"));
        assert!(load_err.to_string().contains("load"));

        let save_err = ConfigError::Save("permission denied".to_string());
        assert!(save_err.to_string().contains("permission denied"));
        assert!(save_err.to_string().contains("save"));
    }
}
