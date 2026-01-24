// SPDX-License-Identifier: GPL-3.0-only

//! Application pages
//!
//! Each page represents a section of the application:
//! - Themes: Theme management and customization
//! - Wallpapers: Wallpaper organization and selection
//! - Screensaver: Screensaver configuration

use serde::{Deserialize, Serialize};

/// Page identifiers
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PageId {
    /// Theme management page
    Themes,
    /// Wallpaper management page
    Wallpapers,
    /// Screensaver configuration page
    Screensaver,
}

impl Default for PageId {
    fn default() -> Self {
        Self::Themes
    }
}

/// Messages from pages
#[derive(Debug, Clone)]
pub enum Message {
    /// Theme page messages
    Themes(ThemesMessage),
    /// Wallpaper page messages
    Wallpapers(WallpapersMessage),
    /// Screensaver page messages
    Screensaver(ScreensaverMessage),
}

/// Theme page messages
#[derive(Debug, Clone)]
pub enum ThemesMessage {
    /// Select a theme
    Select(String),
    /// Apply the selected theme
    Apply,
    /// Export current theme
    Export,
    /// Import a theme file
    Import,
}

/// Wallpaper page messages
#[derive(Debug, Clone)]
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
