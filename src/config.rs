// SPDX-License-Identifier: GPL-3.0-only

//! Application configuration management
//!
//! Uses cosmic-config for persistent storage.

use cosmic_config::CosmicConfigEntry;
use cosmic_config::cosmic_config_derive::CosmicConfigEntry;
use serde::{Deserialize, Serialize};

use crate::pages::PageId;

/// Application identifier for cosmic-config (must match crate::APP_ID)
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
