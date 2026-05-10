// SPDX-License-Identifier: GPL-3.0-only

//! Tool theme synchronization orchestration
//!
//! Manages which tools are enabled for theme sync and orchestrates
//! the sync pipeline: extract colors → save colors.toml → generate tool configs.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::colors::ColorPalette;
use crate::generators;
use crate::paths;

/// Per-tool sync enable/disable flags
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(clippy::struct_excessive_bools)] // Config struct with one bool per tool
pub struct ToolSyncConfig {
    #[serde(default = "default_true")]
    pub ghostty_enabled: bool,
    #[serde(default = "default_true")]
    pub btop_enabled: bool,
    #[serde(default = "default_true")]
    pub nvim_enabled: bool,
    #[serde(default = "default_nvim_colorscheme")]
    pub nvim_colorscheme: String,
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

fn default_nvim_colorscheme() -> String {
    "tokyonight".to_string()
}

impl Default for ToolSyncConfig {
    fn default() -> Self {
        Self {
            ghostty_enabled: true,
            btop_enabled: true,
            nvim_enabled: true,
            nvim_colorscheme: default_nvim_colorscheme(),
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
        let path = paths::tool_sync_config();
        match std::fs::read_to_string(&path) {
            Ok(contents) => toml::from_str(&contents).unwrap_or_default(),
            Err(_) => Self::default(),
        }
    }

    /// Save to `~/.config/cosmic-order/tool-sync.toml`
    pub async fn save(&self) -> Result<(), std::io::Error> {
        let path = paths::tool_sync_config();
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
    /// Names of tools that were successfully synced
    pub synced_tools: Vec<String>,
    pub hooks_result: Option<crate::hooks::HookResults>,
}

impl SyncResult {
    /// Build a human-readable summary of what was synced
    pub fn summary(&self) -> String {
        let mut parts = vec![format!("colors.toml: {}", self.colors_path.display())];
        for name in &self.synced_tools {
            parts.push(format!("{name}: synced"));
        }
        if let Some(ref hr) = self.hooks_result
            && hr.hooks_run > 0
        {
            parts.push(format!("hooks: {}/{} ok", hr.hooks_succeeded, hr.hooks_run));
        }
        parts.join(", ")
    }
}

/// Tool descriptor: (display name, enabled check, sync function)
struct ToolEntry {
    name: &'static str,
    enabled: bool,
    sync_fn: fn(
        &ColorPalette,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<(), String>> + Send + '_>,
    >,
}

/// Sync a tool that only needs `write_theme`
fn write_only<'a, F>(
    palette: &'a ColorPalette,
    name: &'static str,
    write_fn: F,
) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<(), String>> + Send + 'a>>
where
    F: FnOnce(
            &'a ColorPalette,
        ) -> std::pin::Pin<
            Box<dyn std::future::Future<Output = Result<PathBuf, std::io::Error>> + Send + 'a>,
        > + Send
        + 'a,
{
    Box::pin(async move {
        write_fn(palette)
            .await
            .map_err(|e| format!("Failed to write {name} theme: {e}"))?;
        Ok(())
    })
}

/// Sync a tool that needs `write_theme` + `activate_theme`
fn write_and_activate<'a, W, A, Fut>(
    palette: &'a ColorPalette,
    name: &'static str,
    write_fn: W,
    activate_fn: A,
) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<(), String>> + Send + 'a>>
where
    W: FnOnce(
            &'a ColorPalette,
        ) -> std::pin::Pin<
            Box<dyn std::future::Future<Output = Result<PathBuf, std::io::Error>> + Send + 'a>,
        > + Send
        + 'a,
    A: FnOnce() -> Fut + Send + 'a,
    Fut: std::future::Future<Output = Result<(), std::io::Error>> + Send + 'a,
{
    Box::pin(async move {
        write_fn(palette)
            .await
            .map_err(|e| format!("Failed to write {name} theme: {e}"))?;
        activate_fn()
            .await
            .map_err(|e| format!("Failed to activate {name} theme: {e}"))?;
        Ok(())
    })
}

/// Run the full sync pipeline for all enabled tools
pub async fn sync_tools(config: &ToolSyncConfig) -> Result<SyncResult, String> {
    let palette = ColorPalette::from_cosmic();

    let colors_path = palette
        .save()
        .await
        .map_err(|e| format!("Failed to save colors.toml: {e}"))?;

    let tools: Vec<ToolEntry> = vec![
        ToolEntry {
            name: "Ghostty",
            enabled: config.ghostty_enabled,
            sync_fn: |p| {
                write_and_activate(
                    p,
                    "Ghostty",
                    |p| Box::pin(generators::ghostty::write_theme(p)),
                    generators::ghostty::activate_theme,
                )
            },
        },
        ToolEntry {
            name: "btop",
            enabled: config.btop_enabled,
            sync_fn: |p| {
                write_and_activate(
                    p,
                    "btop",
                    |p| Box::pin(generators::btop::write_theme(p)),
                    generators::btop::activate_theme,
                )
            },
        },
        ToolEntry {
            name: "Zellij",
            enabled: config.zellij_enabled,
            sync_fn: |p| {
                write_only(p, "Zellij", |p| {
                    Box::pin(generators::zellij::write_theme(p))
                })
            },
        },
        ToolEntry {
            name: "fzf",
            enabled: config.fzf_enabled,
            sync_fn: |p| write_only(p, "fzf", |p| Box::pin(generators::fzf::write_theme(p))),
        },
        ToolEntry {
            name: "lazygit",
            enabled: config.lazygit_enabled,
            sync_fn: |p| {
                write_only(p, "lazygit", |p| {
                    Box::pin(generators::lazygit::write_theme(p))
                })
            },
        },
    ];

    let mut synced_tools = Vec::new();
    for tool in &tools {
        if tool.enabled {
            (tool.sync_fn)(&palette).await?;
            synced_tools.push(tool.name.to_string());
        }
    }

    // Neovim is synced outside the ToolEntry table because the generator
    // takes a configurable colorscheme name from the user's tool-sync.toml.
    if config.nvim_enabled {
        generators::nvim::write_theme(&palette, &config.nvim_colorscheme)
            .await
            .map_err(|e| format!("Failed to write Neovim theme: {e}"))?;
        synced_tools.push("Neovim".to_string());
    }

    let hooks_result = if config.hooks_enabled {
        Some(crate::hooks::run_hooks(&palette, &colors_path).await)
    } else {
        None
    };

    Ok(SyncResult {
        colors_path,
        synced_tools,
        hooks_result,
    })
}

/// Send SIGUSR2 to a process by name. Returns true if signal was delivered.
fn send_sigusr2(process_name: &str) -> bool {
    match std::process::Command::new("pkill")
        .args(["-USR2", process_name])
        .output()
    {
        Ok(output) if output.status.success() => {
            tracing::debug!("Sent SIGUSR2 to {process_name}");
            true
        }
        Ok(_) => {
            tracing::debug!("No running {process_name} process found");
            false
        }
        Err(e) => {
            tracing::warn!("Failed to signal {process_name}: {e}");
            false
        }
    }
}

/// Send reload signals to running applications after theme sync.
///
/// Best-effort: logs warnings on failure but never returns an error.
#[allow(clippy::cognitive_complexity)]
pub fn signal_running_apps(config: &ToolSyncConfig) -> Vec<String> {
    let mut reloaded = Vec::new();

    // Ghostty & btop: SIGUSR2 triggers config/theme reload
    for (enabled, name) in [
        (config.ghostty_enabled, "Ghostty"),
        (config.btop_enabled, "btop"),
    ] {
        if enabled && send_sigusr2(&name.to_lowercase()) {
            reloaded.push(name.to_string());
        }
    }

    // Neovim: send colorscheme command via --remote-send
    if config.nvim_enabled
        && let Some(runtime_dir) = std::env::var_os("XDG_RUNTIME_DIR")
    {
        let runtime_path = PathBuf::from(&runtime_dir);
        let mut nvim_count = 0u32;
        if let Ok(entries) = std::fs::read_dir(&runtime_path) {
            for entry in entries.flatten() {
                let name = entry.file_name();
                let name_str = name.to_string_lossy();
                if name_str.starts_with("nvim.") && name_str.ends_with(".0") {
                    let socket = entry.path();
                    let cmd = format!(":colorscheme {}<CR>", config.nvim_colorscheme);
                    match std::process::Command::new("nvim")
                        .args(["--server", &socket.to_string_lossy(), "--remote-send", &cmd])
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
                            tracing::warn!("Failed to send to Neovim at {}: {e}", socket.display());
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

    reloaded
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
        assert_eq!(config.nvim_colorscheme, "tokyonight");
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
            nvim_colorscheme: "catppuccin".to_string(),
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
        assert_eq!(deserialized.nvim_colorscheme, "catppuccin");
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
        assert_eq!(config.nvim_colorscheme, "tokyonight");
        assert!(config.zellij_enabled);
        assert!(config.fzf_enabled);
        assert!(!config.fzf_shell_integration);
        assert!(config.lazygit_enabled);
        assert!(config.hooks_enabled);
        assert!(!config.auto_sync);
    }
}
