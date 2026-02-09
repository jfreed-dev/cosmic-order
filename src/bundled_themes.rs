// SPDX-License-Identifier: GPL-3.0-only

//! Bundled community themes from cosmic-themes
//!
//! Embeds RON theme files at compile time and provides a registry
//! for browsing, previewing, and applying community themes.

use std::sync::LazyLock;

use cosmic::cosmic_theme::{Theme as CosmicTheme, ThemeBuilder, ThemeMode};
use cosmic_config::CosmicConfigEntry;
use rust_embed::RustEmbed;

use crate::theme_config::ThemeError;

#[derive(RustEmbed)]
#[folder = "themes/"]
struct BundledThemeFiles;

/// Metadata for a bundled community theme
pub struct BundledTheme {
    /// Index into the registry
    pub index: usize,
    /// Human-readable display name
    pub name: String,
    /// Original filename (e.g. "busy-bee.ron")
    pub filename: String,
    /// Whether this is a dark theme
    pub is_dark: bool,
}

/// Pre-parsed registry of all bundled themes
struct BundledThemeRegistry {
    entries: Vec<(BundledTheme, CosmicTheme)>,
}

/// Parse a single embedded theme file into a `(BundledTheme, CosmicTheme)` pair
fn parse_theme_file(filename: &str, index: usize) -> Option<(BundledTheme, CosmicTheme)> {
    let data = BundledThemeFiles::get(filename)?;

    let content = match std::str::from_utf8(&data.data) {
        Ok(s) => s,
        Err(e) => {
            tracing::warn!("Invalid UTF-8 in theme {filename}: {e}");
            return None;
        }
    };

    let builder: ThemeBuilder = match ron::from_str(content) {
        Ok(b) => b,
        Err(e) => {
            tracing::warn!("Failed to parse theme {filename}: {e}");
            return None;
        }
    };

    let theme = builder.build();

    let meta = BundledTheme {
        index,
        name: display_name_from_filename(filename),
        filename: filename.to_string(),
        is_dark: theme.is_dark,
    };

    Some((meta, theme))
}

static REGISTRY: LazyLock<BundledThemeRegistry> = LazyLock::new(|| {
    let mut filenames: Vec<String> = BundledThemeFiles::iter().map(|f| f.to_string()).collect();
    filenames.sort();

    let entries: Vec<_> = filenames
        .iter()
        .enumerate()
        .filter_map(|(i, filename)| parse_theme_file(filename, i))
        .collect();

    tracing::info!("Loaded {} bundled community themes", entries.len());
    BundledThemeRegistry { entries }
});

/// Get all bundled theme metadata (for building the UI)
pub fn all_themes() -> &'static [(BundledTheme, CosmicTheme)] {
    &REGISTRY.entries
}

/// Get dark community themes only
pub fn dark_themes() -> Vec<&'static (BundledTheme, CosmicTheme)> {
    REGISTRY
        .entries
        .iter()
        .filter(|(meta, _)| meta.is_dark)
        .collect()
}

/// Get light community themes only
pub fn light_themes() -> Vec<&'static (BundledTheme, CosmicTheme)> {
    REGISTRY
        .entries
        .iter()
        .filter(|(meta, _)| !meta.is_dark)
        .collect()
}

/// Apply a bundled theme by index: writes the builder + built theme to system config
pub fn apply_bundled_theme(index: usize) -> Result<(), ThemeError> {
    let (meta, _) = REGISTRY
        .entries
        .get(index)
        .ok_or_else(|| ThemeError::ConfigAccess(format!("Invalid theme index: {index}")))?;

    // Re-parse the builder from the embedded file so we can write it to config
    let data = BundledThemeFiles::get(&meta.filename).ok_or_else(|| {
        ThemeError::ConfigAccess(format!("Missing theme file: {}", meta.filename))
    })?;
    let content = std::str::from_utf8(&data.data)
        .map_err(|e| ThemeError::ConfigAccess(format!("Invalid UTF-8: {e}")))?;
    let builder: ThemeBuilder = ron::from_str(content)
        .map_err(|e| ThemeError::DeserializeError(format!("Failed to parse theme: {e}")))?;

    let is_dark = builder.palette.is_dark();

    // Write the builder to the appropriate config (dark or light)
    let builder_config = if is_dark {
        ThemeBuilder::dark_config()
    } else {
        ThemeBuilder::light_config()
    }
    .map_err(|e| ThemeError::ConfigAccess(e.to_string()))?;

    builder
        .write_entry(&builder_config)
        .map_err(|e| ThemeError::ConfigWrite(e.to_string()))?;

    // Set dark mode to match the theme
    crate::theme_config::ThemeConfig::set_dark_mode(is_dark)?;

    Ok(())
}

/// Snapshot the current system theme (builder + mode) for later restore
pub fn snapshot_current_theme() -> Result<ThemeSnapshot, ThemeError> {
    let mode_config = ThemeMode::config().map_err(|e| ThemeError::ConfigAccess(e.to_string()))?;
    let mode = match ThemeMode::get_entry(&mode_config) {
        Ok(m) | Err((_, m)) => m,
    };

    let is_dark = mode.is_dark;

    let builder_config = if is_dark {
        ThemeBuilder::dark_config()
    } else {
        ThemeBuilder::light_config()
    }
    .map_err(|e| ThemeError::ConfigAccess(e.to_string()))?;

    let builder = match ThemeBuilder::get_entry(&builder_config) {
        Ok(b) | Err((_, b)) => b,
    };

    Ok(ThemeSnapshot { builder, is_dark })
}

/// Restore a previously snapshotted theme
pub fn restore_theme(snapshot: &ThemeSnapshot) -> Result<(), ThemeError> {
    let builder_config = if snapshot.is_dark {
        ThemeBuilder::dark_config()
    } else {
        ThemeBuilder::light_config()
    }
    .map_err(|e| ThemeError::ConfigAccess(e.to_string()))?;

    snapshot
        .builder
        .clone()
        .write_entry(&builder_config)
        .map_err(|e| ThemeError::ConfigWrite(e.to_string()))?;

    crate::theme_config::ThemeConfig::set_dark_mode(snapshot.is_dark)?;

    Ok(())
}

/// Full snapshot of theme state for preview/restore
#[derive(Clone)]
pub struct ThemeSnapshot {
    /// The theme builder that was active
    pub builder: ThemeBuilder,
    /// Whether dark mode was active
    pub is_dark: bool,
}

/// Convert a theme filename to a display name
///
/// `"busy-bee.ron"` → `"Busy Bee"`
pub fn display_name_from_filename(filename: &str) -> String {
    filename
        .trim_end_matches(".ron")
        .split('-')
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                Some(c) => {
                    let upper: String = c.to_uppercase().collect();
                    format!("{upper}{}", chars.as_str())
                }
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn test_display_name_from_filename() {
        assert_eq!(display_name_from_filename("busy-bee.ron"), "Busy Bee");
        assert_eq!(display_name_from_filename("accent-dark.ron"), "Accent Dark");
        assert_eq!(
            display_name_from_filename("ubuntu-classic-dark.ron"),
            "Ubuntu Classic Dark"
        );
        assert_eq!(display_name_from_filename("monokai.ron"), "Monokai");
    }

    #[test]
    fn test_all_themes_parse() {
        let themes = all_themes();
        assert!(!themes.is_empty(), "Should have at least one bundled theme");

        for (meta, theme) in themes {
            assert!(!meta.name.is_empty(), "Theme name should not be empty");
            assert!(
                meta.filename.ends_with(".ron"),
                "Filename should end with .ron"
            );
            // Verify dark/light consistency
            assert_eq!(
                meta.is_dark, theme.is_dark,
                "Metadata dark flag should match theme for {}",
                meta.filename
            );
        }
    }

    #[test]
    fn test_dark_light_split() {
        let dark = dark_themes();
        let light = light_themes();
        let total = all_themes().len();

        assert_eq!(
            dark.len() + light.len(),
            total,
            "Dark + light should equal total"
        );
        assert_eq!(dark.len(), 27, "Expected 27 dark themes");
        assert_eq!(light.len(), 9, "Expected 9 light themes");
    }
}
