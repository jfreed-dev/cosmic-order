// SPDX-License-Identifier: GPL-3.0-only

//! Application pages
//!
//! Each page represents a section of the application:
//! - Themes: Theme management and customization
//! - Wallpapers: Wallpaper organization and selection
//! - Screensaver: Screensaver configuration

use serde::{Deserialize, Serialize};

/// Page identifiers
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum PageId {
    /// Theme management page
    #[default]
    Themes,
    /// Wallpaper management page
    Wallpapers,
    /// Screensaver configuration page
    Screensaver,
}

/// Messages from pages
#[derive(Debug, Clone)]
#[allow(dead_code)] // Variants will be used as features are implemented
pub enum Message {
    /// Theme page messages
    Themes(ThemesMessage),
    /// Wallpaper page messages
    Wallpapers(WallpapersMessage),
    /// Screensaver page messages
    Screensaver(ScreensaverMessage),
}

use crate::theme_config::ThemeId;

/// Theme page messages
#[derive(Debug, Clone)]
pub enum ThemesMessage {
    /// Toggle dark/light mode
    SetDarkMode(bool),
    /// Set accent color (RGB 0.0-1.0)
    SetAccentColor(f32, f32, f32),
    /// Select and apply a theme preset
    SelectTheme(ThemeId),
    /// Export current theme
    Export,
    /// Import a theme file
    Import,
}

/// Wallpaper page messages
#[derive(Debug, Clone)]
#[allow(dead_code)] // Variants will be used in Phase 3
pub enum WallpapersMessage {
    /// Select a wallpaper
    Select(String),
    /// Set as current wallpaper
    SetWallpaper,
    /// Add wallpaper from file
    AddFromFile,
    /// Add wallpaper from URL
    AddFromUrl(String),
    /// Configure rotation
    ConfigureRotation,
}

/// Screensaver page messages
#[derive(Debug, Clone)]
#[allow(dead_code)] // Variants will be used in Phase 4
pub enum ScreensaverMessage {
    /// Set idle timeout
    SetIdleTimeout(u32),
    /// Set lock timeout
    SetLockTimeout(u32),
    /// Set DPMS timeout
    SetDpmsTimeout(u32),
    /// Select logo
    SelectLogo(String),
    /// Enable/disable screensaver
    SetEnabled(bool),
    /// Test screensaver
    Test,
}
