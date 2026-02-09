// SPDX-License-Identifier: GPL-3.0-only

//! Application pages
//!
//! Each page represents a section of the application:
//! - Visuals: Theme customization
//! - Tool Sync: Tool theme sync configuration
//! - Screensaver: Screensaver configuration

use serde::{Deserialize, Serialize};

/// Page identifiers
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum PageId {
    /// Visuals page (themes)
    #[default]
    Visuals,
    /// Tool sync configuration page
    ToolSync,
    /// Screensaver configuration page
    Screensaver,
}

/// Messages from pages
#[derive(Debug, Clone)]
pub enum Message {
    /// Visuals page messages (themes)
    Visuals(ThemesMessage),
    /// Screensaver page messages
    Screensaver(ScreensaverMessage),
}

use crate::theme_config::ThemeId;

/// Theme page messages
#[derive(Debug, Clone)]
pub enum ThemesMessage {
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
    /// Toggle btop theme sync
    SetBtopSync(bool),
    /// Toggle Neovim theme sync
    SetNvimSync(bool),
    /// Toggle Zellij theme sync
    SetZellijSync(bool),
    /// Toggle fzf theme sync
    SetFzfSync(bool),
    /// Toggle fzf shell integration (source line in rc files)
    SetFzfShellIntegration(bool),
    /// Toggle lazygit theme sync
    SetLazygitSync(bool),
    /// Toggle custom hooks
    SetHooksEnabled(bool),
    /// Toggle auto-sync on theme change
    SetAutoSync(bool),
    /// Run theme sync for all enabled tools
    SyncTools,
    /// Sync completed with result
    SyncComplete(Result<String, String>),
    /// Theme creation wizard
    Wizard(WizardMessage),
}

/// Theme creation wizard messages
#[derive(Debug, Clone)]
pub enum WizardMessage {
    /// Open the wizard (snapshot current theme)
    Open,
    /// Close wizard and restore previous theme
    Close,
    /// Move to next step
    NextStep,
    /// Move to previous step
    PrevStep,
    /// Select a bundled theme as base (by registry index)
    SetBaseTheme(usize),
    /// Toggle dark/light mode for the new theme
    SetDarkMode(bool),
    /// Accent color hex input changed
    SetAccentHex(String),
    /// Accent color preset clicked (packed RGB u32)
    SetAccentPreset(u32),
    /// Background color hex input changed
    SetBgHex(String),
    /// Toggle custom background override on/off
    SetBgOverride(bool),
    /// Set outer window gap
    SetOuterGap(u32),
    /// Set inner window gap
    SetInnerGap(u32),
    /// Set active window hint size
    SetActiveHint(u32),
    /// Set corner radii preset index
    SetCornerPreset(usize),
    /// Toggle frosted glass
    SetFrosted(bool),
    /// Theme name text input changed
    SetName(String),
    /// Export theme as RON file (opens file dialog)
    Export,
    /// Export completed
    ExportComplete(Result<String, String>),
    /// Apply theme (keep applied, close wizard, no export)
    Apply,
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
    /// Toggle native session lock
    SetSessionLock(bool),
    /// Save configuration and reload service
    SaveConfig,
    /// Save completed (bool = launch test after save)
    SaveComplete(Result<(), String>, bool),
    /// Save and launch screensaver test
    SaveAndTest,
    /// Screensaver test process exited
    ScreensaverTestExited(Result<(), String>),
}
