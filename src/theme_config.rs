// SPDX-License-Identifier: GPL-3.0-only

//! Theme configuration reading and writing
//!
//! Reads and modifies COSMIC theme settings via cosmic-theme/cosmic-config.

use std::path::Path;

use cosmic::cosmic_theme::palette::{Srgb, Srgba};
use cosmic::cosmic_theme::{Theme as CosmicTheme, ThemeBuilder, ThemeMode};
use cosmic::theme;
use cosmic_config::CosmicConfigEntry;

/// Theme operation errors
#[derive(Debug, thiserror::Error)]
pub enum ThemeError {
    #[error("Failed to access theme config: {0}")]
    ConfigAccess(String),
    #[error("Failed to write theme config: {0}")]
    ConfigWrite(String),
    #[error("Failed to serialize theme: {0}")]
    SerializeError(String),
    #[error("Failed to write theme file: {0}")]
    FileWriteError(String),
    #[error("Failed to read theme file: {0}")]
    FileReadError(String),
    #[error("Failed to deserialize theme: {0}")]
    DeserializeError(String),
}

/// Theme identifier (built-in or bundled community theme)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThemeId {
    Dark,
    Light,
    HighContrastDark,
    HighContrastLight,
    /// Bundled community theme by index
    Bundled(usize),
}

/// Snapshot of theme state for preview restore
#[derive(Clone)]
pub struct ThemePreviewState {
    pub config: ThemeConfig,
    pub previewing_id: ThemeId,
    /// Full theme snapshot for proper restore (includes builder state)
    pub snapshot: Option<crate::bundled_themes::ThemeSnapshot>,
}

/// Theme preview information
#[derive(Debug, Clone)]
pub struct ThemePreview {
    /// Theme identifier
    pub id: ThemeId,
    /// Display name
    pub name: String,
    /// Whether this is a dark theme
    pub is_dark: bool,
}

impl ThemePreview {
    /// Get all built-in theme previews
    pub fn built_in_themes() -> Vec<Self> {
        vec![
            Self::from_cosmic_theme(ThemeId::Dark, &CosmicTheme::dark_default()),
            Self::from_cosmic_theme(ThemeId::Light, &CosmicTheme::light_default()),
            Self::from_cosmic_theme(
                ThemeId::HighContrastDark,
                &CosmicTheme::high_contrast_dark_default(),
            ),
            Self::from_cosmic_theme(
                ThemeId::HighContrastLight,
                &CosmicTheme::high_contrast_light_default(),
            ),
        ]
    }

    fn from_cosmic_theme(id: ThemeId, theme: &CosmicTheme) -> Self {
        Self {
            id,
            name: theme.name.clone(),
            is_dark: theme.is_dark,
        }
    }

    /// Apply this theme preset
    pub fn apply(&self) -> Result<(), ThemeError> {
        // Set the dark mode based on theme
        ThemeConfig::set_dark_mode(self.is_dark)?;

        // Note: High contrast themes would need additional handling
        // For now, we just switch between dark/light
        // TODO: Add high contrast toggle support

        Ok(())
    }
}

/// Theme configuration extracted from COSMIC
#[derive(Debug, Clone)]
pub struct ThemeConfig {
    /// Theme name
    pub name: String,
    /// Whether dark mode is active
    pub is_dark: bool,
    /// Accent color (RGBA)
    pub accent_color: Srgba,
}

impl Default for ThemeConfig {
    fn default() -> Self {
        Self {
            name: "Unknown".to_string(),
            is_dark: true,
            accent_color: Srgba::new(0.39, 0.82, 0.87, 1.0),
        }
    }
}

impl ThemeConfig {
    /// Load current theme configuration from COSMIC
    pub fn load() -> Self {
        let active_theme = theme::active();
        let cosmic = active_theme.cosmic();

        Self {
            name: cosmic.name.clone(),
            is_dark: cosmic.is_dark,
            accent_color: cosmic.accent.base,
        }
    }

    /// Set dark mode on or off
    pub fn set_dark_mode(is_dark: bool) -> Result<(), ThemeError> {
        let config = ThemeMode::config().map_err(|e| ThemeError::ConfigAccess(e.to_string()))?;

        let mut mode = match ThemeMode::get_entry(&config) {
            Ok(m) => m,
            Err((errors, m)) => {
                for err in errors {
                    if err.is_err() {
                        tracing::warn!("ThemeMode load warning: {err}");
                    }
                }
                m
            }
        };

        mode.is_dark = is_dark;
        mode.write_entry(&config)
            .map_err(|e| ThemeError::ConfigWrite(e.to_string()))
    }

    /// Set accent color
    pub fn set_accent_color(r: f32, g: f32, b: f32, is_dark: bool) -> Result<(), ThemeError> {
        let color = Srgb::new(r, g, b);

        // Get the appropriate builder config based on current mode
        let config = if is_dark {
            ThemeBuilder::dark_config()
        } else {
            ThemeBuilder::light_config()
        }
        .map_err(|e| ThemeError::ConfigAccess(e.to_string()))?;

        let mut builder = match ThemeBuilder::get_entry(&config) {
            Ok(b) => b,
            Err((errors, b)) => {
                for err in errors {
                    if err.is_err() {
                        tracing::warn!("ThemeBuilder load warning: {err}");
                    }
                }
                b
            }
        };

        // Set the accent color and persist
        builder
            .set_accent(&config, Some(color))
            .map_err(|e| ThemeError::ConfigWrite(e.to_string()))?;

        Ok(())
    }

    /// Export the current theme to a RON file at the given path
    pub async fn export_theme(path: &Path) -> Result<String, ThemeError> {
        let cosmic_theme = theme::active().cosmic().clone();

        let pretty = ron::ser::PrettyConfig::default();
        let serialized = ron::ser::to_string_pretty(&cosmic_theme, pretty)
            .map_err(|e| ThemeError::SerializeError(e.to_string()))?;

        tokio::fs::write(path, &serialized)
            .await
            .map_err(|e| ThemeError::FileWriteError(e.to_string()))?;

        let path_str = path.to_string_lossy().to_string();
        Ok(path_str)
    }

    /// Import a theme from a RON file and apply it to the system
    pub async fn import_theme(path: &Path) -> Result<String, ThemeError> {
        let contents = tokio::fs::read_to_string(path)
            .await
            .map_err(|e| ThemeError::FileReadError(e.to_string()))?;

        let imported: CosmicTheme =
            ron::from_str(&contents).map_err(|e| ThemeError::DeserializeError(e.to_string()))?;

        let config = if imported.is_dark {
            CosmicTheme::dark_config()
        } else {
            CosmicTheme::light_config()
        }
        .map_err(|e| ThemeError::ConfigAccess(e.to_string()))?;

        imported
            .write_entry(&config)
            .map_err(|e| ThemeError::ConfigWrite(e.to_string()))?;

        let path_str = path.to_string_lossy().to_string();
        Ok(path_str)
    }

    /// Generate a default filename for exporting the current theme
    pub fn default_export_filename() -> String {
        let active_theme = theme::active();
        let name = &active_theme.cosmic().name;
        let sanitized = name.to_lowercase().replace(' ', "-");
        format!("{sanitized}.ron")
    }

    /// Write a `ThemeBuilder` to the appropriate dark/light cosmic-config entry
    pub fn write_builder(builder: &ThemeBuilder, is_dark: bool) -> Result<(), ThemeError> {
        let config = if is_dark {
            ThemeBuilder::dark_config()
        } else {
            ThemeBuilder::light_config()
        }
        .map_err(|e| ThemeError::ConfigAccess(e.to_string()))?;

        builder
            .clone()
            .write_entry(&config)
            .map_err(|e| ThemeError::ConfigWrite(e.to_string()))
    }

    /// Format a color as hex string
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    pub fn color_to_hex(color: &Srgba) -> String {
        format!(
            "#{:02X}{:02X}{:02X}",
            (color.red * 255.0) as u8,
            (color.green * 255.0) as u8,
            (color.blue * 255.0) as u8
        )
    }

    /// Format a color as RGB string
    #[allow(dead_code, clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    pub fn color_to_rgb(color: &Srgba) -> String {
        format!(
            "rgb({}, {}, {})",
            (color.red * 255.0) as u8,
            (color.green * 255.0) as u8,
            (color.blue * 255.0) as u8
        )
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn test_color_to_hex() {
        let color = Srgba::new(1.0, 0.5, 0.0, 1.0);
        assert_eq!(ThemeConfig::color_to_hex(&color), "#FF7F00");
    }

    #[test]
    fn test_color_to_rgb() {
        let color = Srgba::new(1.0, 0.5, 0.0, 1.0);
        assert_eq!(ThemeConfig::color_to_rgb(&color), "rgb(255, 127, 0)");
    }
}
