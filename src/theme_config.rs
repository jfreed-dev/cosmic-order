// SPDX-License-Identifier: GPL-3.0-only

//! Theme configuration reading and writing
//!
//! Reads and modifies COSMIC theme settings via cosmic-theme/cosmic-config.

use cosmic::cosmic_theme::palette::{Srgb, Srgba};
use cosmic::cosmic_theme::{ThemeBuilder, ThemeMode};
use cosmic::theme;
use cosmic_config::CosmicConfigEntry;

/// Theme operation errors
#[derive(Debug, thiserror::Error)]
pub enum ThemeError {
    #[error("Failed to access theme config: {0}")]
    ConfigAccess(String),
    #[error("Failed to write theme config: {0}")]
    ConfigWrite(String),
}

/// Theme configuration extracted from COSMIC
#[derive(Debug, Clone)]
#[allow(dead_code)] // Fields will be used as UI expands
pub struct ThemeConfig {
    /// Theme name
    pub name: String,
    /// Whether dark mode is active
    pub is_dark: bool,
    /// Accent color (RGBA)
    pub accent_color: Srgba,
    /// Background color (RGBA)
    pub background_color: Srgba,
    /// Primary text color (RGBA)
    pub text_color: Srgba,
}

impl Default for ThemeConfig {
    fn default() -> Self {
        Self {
            name: "Unknown".to_string(),
            is_dark: true,
            accent_color: Srgba::new(0.39, 0.82, 0.87, 1.0),
            background_color: Srgba::new(0.11, 0.11, 0.11, 1.0),
            text_color: Srgba::new(1.0, 1.0, 1.0, 1.0),
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
            background_color: cosmic.background.base,
            text_color: cosmic.primary.on,
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

    /// Format a color as hex string
    pub fn color_to_hex(color: &Srgba) -> String {
        format!(
            "#{:02X}{:02X}{:02X}",
            (color.red * 255.0) as u8,
            (color.green * 255.0) as u8,
            (color.blue * 255.0) as u8
        )
    }

    /// Format a color as RGB string
    #[allow(dead_code)] // Available for future use
    pub fn color_to_rgb(color: &Srgba) -> String {
        format!(
            "rgb({}, {}, {})",
            (color.red * 255.0) as u8,
            (color.green * 255.0) as u8,
            (color.blue * 255.0) as u8
        )
    }

    /// Get accent color as hex
    pub fn accent_hex(&self) -> String {
        Self::color_to_hex(&self.accent_color)
    }

    /// Get background color as hex
    pub fn background_hex(&self) -> String {
        Self::color_to_hex(&self.background_color)
    }

    /// Get text color as hex
    pub fn text_hex(&self) -> String {
        Self::color_to_hex(&self.text_color)
    }
}

#[cfg(test)]
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
