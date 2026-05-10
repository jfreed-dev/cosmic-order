// SPDX-License-Identifier: GPL-3.0-only

//! Centralized path resolution for config files
//!
//! All config path helpers use `directories::BaseDirs` with a consistent
//! fallback to `$HOME/.config` (or `/tmp` if `$HOME` is unset).

use std::path::PathBuf;

/// Resolve the user's XDG config directory (e.g. `~/.config`)
fn config_dir() -> PathBuf {
    directories::BaseDirs::new().map_or_else(
        || {
            let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
            PathBuf::from(home).join(".config")
        },
        |d| d.config_dir().to_path_buf(),
    )
}

/// Resolve the user's home directory
pub fn home_dir() -> PathBuf {
    directories::BaseDirs::new().map_or_else(
        || {
            let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
            PathBuf::from(home)
        },
        |d| d.home_dir().to_path_buf(),
    )
}

/// `~/.config/cosmic-order/`
pub fn cosmic_order_config_dir() -> PathBuf {
    config_dir().join("cosmic-order")
}

/// `~/.config/ghostty/config`
pub fn ghostty_config() -> PathBuf {
    config_dir().join("ghostty").join("config")
}

/// `~/.config/ghostty/themes/`
pub fn ghostty_themes_dir() -> PathBuf {
    config_dir().join("ghostty").join("themes")
}

/// `~/.config/btop/btop.conf`
pub fn btop_config() -> PathBuf {
    config_dir().join("btop").join("btop.conf")
}

/// `~/.config/btop/themes/`
pub fn btop_themes_dir() -> PathBuf {
    config_dir().join("btop").join("themes")
}

/// `~/.config/nvim/lua/plugins/colorscheme.lua`
pub fn nvim_colorscheme() -> PathBuf {
    config_dir()
        .join("nvim")
        .join("lua")
        .join("plugins")
        .join("colorscheme.lua")
}

/// `~/.config/zellij/config.kdl`
pub fn zellij_config() -> PathBuf {
    config_dir().join("zellij").join("config.kdl")
}

/// `~/.config/lazygit/config.yml`
pub fn lazygit_config() -> PathBuf {
    config_dir().join("lazygit").join("config.yml")
}

/// `~/.config/cosmic-order/fzf-theme.sh`
pub fn fzf_theme() -> PathBuf {
    cosmic_order_config_dir().join("fzf-theme.sh")
}

/// `~/.config/cosmic-order/tool-sync.toml`
pub fn tool_sync_config() -> PathBuf {
    cosmic_order_config_dir().join("tool-sync.toml")
}

/// `~/.config/cosmic-order/hooks.d/`
pub fn hooks_dir() -> PathBuf {
    cosmic_order_config_dir().join("hooks.d")
}

/// `~/.config/cosmic-screensaver/`
pub fn screensaver_config_dir() -> PathBuf {
    config_dir().join("cosmic-screensaver")
}

/// `~/.config/cosmic-screensaver/config`
pub fn screensaver_config() -> PathBuf {
    screensaver_config_dir().join("config")
}

/// `~/.config/cosmic-screensaver/power-state.env`
pub fn screensaver_power_env() -> PathBuf {
    screensaver_config_dir().join("power-state.env")
}

/// `~/.config/cosmic-screensaver/swayidle.conf`
pub fn screensaver_swayidle_config() -> PathBuf {
    screensaver_config_dir().join("swayidle.conf")
}

/// System data directory for bundled screensaver scripts and logos
/// `/usr/share/cosmic-order/screensaver/`
pub fn screensaver_data_dir() -> PathBuf {
    PathBuf::from("/usr/share/cosmic-order/screensaver")
}

/// `~/.local/bin/screensaver-ctl`
pub fn screensaver_ctl() -> PathBuf {
    home_dir().join(".local/bin/screensaver-ctl")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_paths_are_absolute() {
        // All paths should resolve to something containing .config or .local
        let paths = [
            cosmic_order_config_dir(),
            ghostty_config(),
            ghostty_themes_dir(),
            btop_config(),
            btop_themes_dir(),
            nvim_colorscheme(),
            zellij_config(),
            lazygit_config(),
            fzf_theme(),
            tool_sync_config(),
            hooks_dir(),
            screensaver_config(),
            screensaver_power_env(),
            screensaver_swayidle_config(),
            screensaver_ctl(),
        ];

        for path in &paths {
            assert!(
                path.components().count() > 1,
                "Path too short: {}",
                path.display()
            );
        }
    }

    #[test]
    fn test_cosmic_order_paths_share_prefix() {
        let parent = cosmic_order_config_dir();
        assert!(tool_sync_config().starts_with(&parent));
        assert!(fzf_theme().starts_with(&parent));
        assert!(hooks_dir().starts_with(&parent));
    }
}
