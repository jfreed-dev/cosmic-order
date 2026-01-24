// SPDX-License-Identifier: GPL-3.0-only

//! Application configuration management
//!
//! Uses cosmic-config for persistent storage.

use serde::{Deserialize, Serialize};

use crate::pages::PageId;

/// Application configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Last active page
    pub active_page: PageId,
    /// Window width
    pub window_width: u32,
    /// Window height
    pub window_height: u32,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            active_page: PageId::Themes,
            window_width: 900,
            window_height: 600,
        }
    }
}

impl Config {
    /// Load configuration from cosmic-config
    pub fn load() -> Result<Self, ConfigError> {
        // TODO: Implement cosmic-config loading
        // For now, return default
        Ok(Self::default())
    }

    /// Save configuration to cosmic-config
    #[allow(dead_code)] // Will be used when cosmic-config integration is complete
    #[allow(clippy::unused_self)] // Will use self when cosmic-config saving is implemented
    #[allow(clippy::missing_const_for_fn)] // Will not be const once implemented
    pub fn save(&self) -> Result<(), ConfigError> {
        // TODO: Implement cosmic-config saving
        Ok(())
    }
}

/// Configuration errors
#[derive(Debug, thiserror::Error)]
#[allow(dead_code)] // Variants will be used when cosmic-config integration is complete
pub enum ConfigError {
    #[error("Failed to load configuration: {0}")]
    Load(String),
    #[error("Failed to save configuration: {0}")]
    Save(String),
}
