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
    #[serde(default = "default_true")]
    pub btop_enabled: bool,
    #[serde(default = "default_true")]
    pub nvim_enabled: bool,
    #[serde(default = "default_true")]
    pub zellij_enabled: bool,
    #[serde(default = "default_true")]
    pub fzf_enabled: bool,
    #[serde(default = "default_false")]
    pub fzf_shell_integration: bool,
    #[serde(default = "default_true")]
    pub lazygit_enabled: bool,
    #[serde(default = "default_true")]
    pub hooks_enabled: bool,
    #[serde(default = "default_false")]
    pub auto_sync: bool,
}

const fn default_true() -> bool {
    true
}

const fn default_false() -> bool {
    false
}

impl Default for ToolSyncConfig {
    fn default() -> Self {
        Self {
            ghostty_enabled: true,
            btop_enabled: true,
            nvim_enabled: true,
            zellij_enabled: true,
            fzf_enabled: true,
            fzf_shell_integration: false,
            lazygit_enabled: true,
            hooks_enabled: true,
            auto_sync: false,
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
    pub btop_synced: bool,
    pub nvim_synced: bool,
    pub zellij_synced: bool,
    pub fzf_synced: bool,
    pub lazygit_synced: bool,
    pub hooks_result: Option<crate::hooks::HookResults>,
}

/// Run the full sync pipeline for all enabled tools
pub async fn sync_tools(config: &ToolSyncConfig) -> Result<SyncResult, String> {
    let palette = ColorPalette::from_cosmic();

    let colors_path = palette
        .save()
        .await
        .map_err(|e| format!("Failed to save colors.toml: {e}"))?;

    let mut ghostty_synced = false;
    let mut btop_synced = false;
    let mut nvim_synced = false;
    let mut zellij_synced = false;
    let mut fzf_synced = false;
    let mut lazygit_synced = false;

    if config.ghostty_enabled {
        generators::ghostty::write_theme(&palette)
            .await
            .map_err(|e| format!("Failed to write Ghostty theme: {e}"))?;

        generators::ghostty::activate_theme()
            .await
            .map_err(|e| format!("Failed to activate Ghostty theme: {e}"))?;

        ghostty_synced = true;
    }

    if config.btop_enabled {
        generators::btop::write_theme(&palette)
            .await
            .map_err(|e| format!("Failed to write btop theme: {e}"))?;

        generators::btop::activate_theme()
            .await
            .map_err(|e| format!("Failed to activate btop theme: {e}"))?;

        btop_synced = true;
    }

    if config.nvim_enabled {
        generators::nvim::write_theme(&palette)
            .await
            .map_err(|e| format!("Failed to write Neovim colorscheme: {e}"))?;

        nvim_synced = true;
    }

    if config.zellij_enabled {
        generators::zellij::write_theme(&palette)
            .await
            .map_err(|e| format!("Failed to write Zellij theme: {e}"))?;

        zellij_synced = true;
    }

    if config.fzf_enabled {
        generators::fzf::write_theme(&palette)
            .await
            .map_err(|e| format!("Failed to write fzf theme: {e}"))?;

        fzf_synced = true;
    }

    if config.lazygit_enabled {
        generators::lazygit::write_theme(&palette)
            .await
            .map_err(|e| format!("Failed to write lazygit theme: {e}"))?;

        lazygit_synced = true;
    }

    let hooks_result = if config.hooks_enabled {
        Some(crate::hooks::run_hooks(&palette, &colors_path).await)
    } else {
        None
    };

    Ok(SyncResult {
        colors_path,
        ghostty_synced,
        btop_synced,
        nvim_synced,
        zellij_synced,
        fzf_synced,
        lazygit_synced,
        hooks_result,
    })
}

/// Send reload signals to running applications after theme sync.
///
/// Best-effort: logs warnings on failure but never returns an error.
pub async fn signal_running_apps(config: &ToolSyncConfig) -> Vec<String> {
    let mut reloaded = Vec::new();

    // Ghostty: SIGUSR2 triggers config reload
    if config.ghostty_enabled {
        match std::process::Command::new("pkill")
            .args(["-USR2", "ghostty"])
            .output()
        {
            Ok(output) if output.status.success() => {
                tracing::debug!("Sent SIGUSR2 to Ghostty");
                reloaded.push("Ghostty".to_string());
            }
            Ok(_) => tracing::debug!("No running Ghostty process found"),
            Err(e) => tracing::warn!("Failed to signal Ghostty: {e}"),
        }
    }

    // btop: SIGUSR2 triggers theme reload
    if config.btop_enabled {
        match std::process::Command::new("pkill")
            .args(["-USR2", "btop"])
            .output()
        {
            Ok(output) if output.status.success() => {
                tracing::debug!("Sent SIGUSR2 to btop");
                reloaded.push("btop".to_string());
            }
            Ok(_) => tracing::debug!("No running btop process found"),
            Err(e) => tracing::warn!("Failed to signal btop: {e}"),
        }
    }

    // Neovim: send colorscheme command via --remote-send
    if config.nvim_enabled {
        if let Some(runtime_dir) = std::env::var_os("XDG_RUNTIME_DIR") {
            let runtime_path = PathBuf::from(&runtime_dir);
            let mut nvim_count = 0u32;
            if let Ok(entries) = std::fs::read_dir(&runtime_path) {
                for entry in entries.flatten() {
                    let name = entry.file_name();
                    let name_str = name.to_string_lossy();
                    if name_str.starts_with("nvim.") && name_str.ends_with(".0") {
                        let socket = entry.path();
                        match std::process::Command::new("nvim")
                            .args([
                                "--server",
                                &socket.to_string_lossy(),
                                "--remote-send",
                                ":colorscheme tokyonight<CR>",
                            ])
                            .output()
                        {
                            Ok(output) if output.status.success() => {
                                nvim_count += 1;
                            }
                            Ok(output) => {
                                tracing::warn!(
                                    "Neovim remote-send failed for {}: {}",
                                    socket.display(),
                                    String::from_utf8_lossy(&output.stderr)
                                );
                            }
                            Err(e) => {
                                tracing::warn!(
                                    "Failed to send to Neovim at {}: {e}",
                                    socket.display()
                                );
                            }
                        }
                    }
                }
            }
            if nvim_count > 0 {
                tracing::debug!("Reloaded {nvim_count} Neovim instance(s)");
                reloaded.push(if nvim_count == 1 {
                    "Neovim".to_string()
                } else {
                    format!("Neovim ({nvim_count})")
                });
            }
        }
    }

    reloaded
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
        assert!(config.btop_enabled);
        assert!(config.nvim_enabled);
        assert!(config.zellij_enabled);
        assert!(config.fzf_enabled);
        assert!(!config.fzf_shell_integration);
        assert!(config.lazygit_enabled);
        assert!(config.hooks_enabled);
        assert!(!config.auto_sync);
    }

    #[test]
    fn test_config_roundtrip() {
        let config = ToolSyncConfig {
            ghostty_enabled: false,
            btop_enabled: true,
            nvim_enabled: false,
            zellij_enabled: true,
            fzf_enabled: true,
            fzf_shell_integration: true,
            lazygit_enabled: false,
            hooks_enabled: true,
            auto_sync: true,
        };
        let serialized = toml::to_string_pretty(&config).unwrap();
        let deserialized: ToolSyncConfig = toml::from_str(&serialized).unwrap();
        assert!(!deserialized.ghostty_enabled);
        assert!(deserialized.btop_enabled);
        assert!(!deserialized.nvim_enabled);
        assert!(deserialized.zellij_enabled);
        assert!(deserialized.fzf_enabled);
        assert!(deserialized.fzf_shell_integration);
        assert!(!deserialized.lazygit_enabled);
        assert!(deserialized.hooks_enabled);
        assert!(deserialized.auto_sync);
    }

    #[test]
    fn test_config_deserialize_missing_fields() {
        // Simulate old config file with only ghostty_enabled
        let old_config = "ghostty_enabled = false\n";
        let config: ToolSyncConfig = toml::from_str(old_config).unwrap();
        assert!(!config.ghostty_enabled);
        // New fields should default to true (except fzf_shell_integration)
        assert!(config.btop_enabled);
        assert!(config.nvim_enabled);
        assert!(config.zellij_enabled);
        assert!(config.fzf_enabled);
        assert!(!config.fzf_shell_integration);
        assert!(config.lazygit_enabled);
        assert!(config.hooks_enabled);
        assert!(!config.auto_sync);
    }
}
