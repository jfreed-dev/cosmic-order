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
    /// Select and apply a theme preset (retained for programmatic use)
    #[allow(dead_code)]
    SelectTheme(ThemeId),
    /// Export current theme
    Export,
    /// Export completed with result (path or error message)
    ExportComplete(Result<String, String>),
    /// Import a theme file
    Import,
    /// Import completed with result (path or error message)
    ImportComplete(Result<String, String>),
    /// Preview a theme (apply temporarily with confirm/cancel)
    PreviewTheme(ThemeId),
    /// Confirm the previewed theme (keep it applied)
    ConfirmPreview,
    /// Cancel the preview and restore the previous theme
    CancelPreview,
}

/// Wallpaper page messages
#[derive(Debug, Clone)]
pub enum WallpapersMessage {
    /// Switch collection view (None = "All")
    SelectCollection(Option<String>),
    /// Highlight a wallpaper by full path
    SelectWallpaper(String),
    /// Apply the selected wallpaper to the desktop
    ApplyWallpaper,
    /// Apply completed with result (path or error)
    ApplyComplete(Result<String, String>),
    /// Toggle rotation on/off
    SetRotationEnabled(bool),
    /// Set rotation frequency in seconds
    SetRotationFrequency(u32),
    /// Set the scaling mode
    SetScalingMode(usize),
    /// Save rotation/scaling settings to disk
    SaveSettings,
    /// Save completed with result
    SaveComplete(Result<(), String>),
    /// Open file picker to import a wallpaper
    ImportFromFile,
    /// Import completed with result (path or error)
    ImportComplete(Result<String, String>),
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
