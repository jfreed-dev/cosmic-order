// SPDX-License-Identifier: GPL-3.0-only

//! Tool theme synchronization orchestration
//!
//! Manages which tools are enabled for theme sync and orchestrates
//! the sync pipeline: extract colors → save colors.toml → generate tool configs.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::colors::ColorPalette;
use crate::generators;

/// Per-tool sync enable/disable flags
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolSyncConfig {
    #[serde(default = "default_true")]
    pub ghostty_enabled: bool,
}

const fn default_true() -> bool {
    true
}

impl Default for ToolSyncConfig {
    fn default() -> Self {
        Self {
            ghostty_enabled: true,
        }
    }
}

impl ToolSyncConfig {
    /// Load from `~/.config/cosmic-order/tool-sync.toml`
    pub fn load() -> Self {
        let path = config_path();
        match std::fs::read_to_string(&path) {
            Ok(contents) => toml::from_str(&contents).unwrap_or_default(),
            Err(_) => Self::default(),
        }
    }

    /// Save to `~/.config/cosmic-order/tool-sync.toml`
    pub async fn save(&self) -> Result<(), std::io::Error> {
        let path = config_path();
        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }
        let contents = toml::to_string_pretty(self)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string()))?;
        tokio::fs::write(&path, contents).await
    }
}

/// Result of a sync operation
#[derive(Debug, Clone)]
pub struct SyncResult {
    pub colors_path: PathBuf,
    pub ghostty_synced: bool,
}

/// Run the full sync pipeline for all enabled tools
pub async fn sync_tools(config: &ToolSyncConfig) -> Result<SyncResult, String> {
    let palette = ColorPalette::from_cosmic();

    let colors_path = palette
        .save()
        .await
        .map_err(|e| format!("Failed to save colors.toml: {e}"))?;

    let mut ghostty_synced = false;

    if config.ghostty_enabled {
        generators::ghostty::write_theme(&palette)
            .await
            .map_err(|e| format!("Failed to write Ghostty theme: {e}"))?;

        generators::ghostty::activate_theme()
            .await
            .map_err(|e| format!("Failed to activate Ghostty theme: {e}"))?;

        ghostty_synced = true;
    }

    Ok(SyncResult {
        colors_path,
        ghostty_synced,
    })
}

fn config_path() -> PathBuf {
    directories::BaseDirs::new()
        .map(|d| d.config_dir().join("cosmic-order").join("tool-sync.toml"))
        .unwrap_or_else(|| {
            let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
            PathBuf::from(home).join(".config/cosmic-order/tool-sync.toml")
        })
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = ToolSyncConfig::default();
        assert!(config.ghostty_enabled);
    }

    #[test]
    fn test_config_roundtrip() {
        let config = ToolSyncConfig {
            ghostty_enabled: false,
        };
        let serialized = toml::to_string_pretty(&config).unwrap();
        let deserialized: ToolSyncConfig = toml::from_str(&serialized).unwrap();
        assert!(!deserialized.ghostty_enabled);
    }
}
