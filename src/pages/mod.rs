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
    /// Toggle Ghostty theme sync
    SetGhosttySync(bool),
    /// Run theme sync for all enabled tools
    SyncTools,
    /// Sync completed with result
    SyncComplete(Result<String, String>),
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
    /// Show next page of wallpaper thumbnails
    GridNextPage,
    /// Show previous page of wallpaper thumbnails
    GridPrevPage,
    /// Background thumbnail generation completed for the visible page
    ThumbnailsReady,
}

/// Screensaver page messages
#[derive(Debug, Clone)]
pub enum ScreensaverMessage {
    /// Enable/disable screensaver
    SetEnabled(bool),
    /// Set idle timeout (seconds)
    SetIdleTimeout(u32),
    /// Set lock timeout (seconds)
    SetLockTimeout(u32),
    /// Set DPMS timeout (seconds)
    SetDpmsTimeout(u32),
    /// Set frame rate (dropdown index)
    SetFrameRate(usize),
    /// Set exclude effects text
    SetExcludeEffects(String),
    /// Set include effects text
    SetIncludeEffects(String),
    /// Set fade in effect (dropdown index)
    SetFadeInEffect(usize),
    /// Set fade out effect (dropdown index)
    SetFadeOutEffect(usize),
    /// Toggle clock display
    SetShowClock(bool),
    /// Set clock duration (dropdown index)
    SetClockDuration(usize),
    /// Set clock format (dropdown index)
    SetClockFormat(usize),
    /// Set terminal emulator (dropdown index)
    SetTerminal(usize),
    /// Select a logo from the grid by its file path
    SelectLogo(String),
    /// Open file dialog to select logo
    SelectLogoDialog,
    /// Logo selection completed
    SelectLogoComplete(Result<String, String>),
    /// Toggle cursor hiding
    SetCursorHide(bool),
    /// Toggle mouse pointer hiding
    SetHideMouse(bool),
    /// Toggle dismiss on keypress
    SetDismissOnKey(bool),
    /// Save configuration and reload service
    SaveConfig,
    /// Save completed (bool = launch test after save)
    SaveComplete(Result<(), String>, bool),
    /// Save and launch screensaver test
    SaveAndTest,
    /// Screensaver test process exited
    ScreensaverTestExited(Result<(), String>),
}
