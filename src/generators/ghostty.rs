// SPDX-License-Identifier: GPL-3.0-only

//! Ghostty terminal theme generator
//!
//! Generates a Ghostty theme file from the COSMIC color palette
//! and activates it in the user's Ghostty config.

use std::fmt::Write;
use std::path::PathBuf;

use crate::colors::ColorPalette;
use crate::paths;

/// Generate Ghostty theme file content from a color palette
pub fn generate_theme(palette: &ColorPalette) -> String {
    let mut out = String::with_capacity(512);
    let _ = writeln!(out, "background = {}", palette.background);
    let _ = writeln!(out, "foreground = {}", palette.foreground);
    let _ = writeln!(out, "cursor-color = {}", palette.cursor);
    let _ = writeln!(
        out,
        "selection-foreground = {}",
        palette.selection_foreground
    );
    let _ = writeln!(
        out,
        "selection-background = {}",
        palette.selection_background
    );

    for (i, color) in palette.colors.iter().enumerate() {
        let _ = writeln!(out, "palette = {i}={color}");
    }

    out
}

/// Write the generated theme to `~/.config/ghostty/themes/cosmic-synced`
pub async fn write_theme(palette: &ColorPalette) -> Result<PathBuf, std::io::Error> {
    let themes_dir = paths::ghostty_themes_dir();
    tokio::fs::create_dir_all(&themes_dir).await?;
    let path = themes_dir.join("cosmic-synced");
    tokio::fs::write(&path, generate_theme(palette)).await?;
    Ok(path)
}

/// Color-related config keys that conflict with theme files.
/// When a theme is active, these inline settings override it,
/// so they must be removed.
const COLOR_KEYS: &[&str] = &[
    "background",
    "foreground",
    "cursor-color",
    "selection-foreground",
    "selection-background",
    "palette",
];

/// Activate the `cosmic-synced` theme in `~/.config/ghostty/config`
///
/// Replaces any existing `theme =` line (or appends one) and removes
/// inline color settings that would override the theme file.
pub async fn activate_theme() -> Result<(), std::io::Error> {
    let config_path = paths::ghostty_config();

    // Ensure parent dir exists
    if let Some(parent) = config_path.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }

    let contents = match tokio::fs::read_to_string(&config_path).await {
        Ok(c) => c,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => String::new(),
        Err(e) => return Err(e),
    };

    let theme_line = "theme = cosmic-synced";
    let mut found = false;
    let mut new_lines: Vec<String> = Vec::new();

    for line in contents.lines() {
        let trimmed = line.trim_start();

        // Replace existing theme line
        if trimmed.starts_with("theme") && trimmed.contains('=') {
            new_lines.push(theme_line.to_string());
            found = true;
            continue;
        }

        // Strip inline color settings that would override the theme
        let is_color_key = COLOR_KEYS.iter().any(|key| {
            trimmed.starts_with(key) && trimmed[key.len()..].trim_start().starts_with('=')
        });
        if is_color_key {
            continue;
        }

        new_lines.push(line.to_string());
    }

    // Collapse runs of 3+ blank lines left by removed color settings
    let mut collapsed: Vec<String> = Vec::with_capacity(new_lines.len());
    let mut blank_run = 0;
    for line in &new_lines {
        if line.is_empty() {
            blank_run += 1;
            if blank_run <= 2 {
                collapsed.push(line.clone());
            }
        } else {
            blank_run = 0;
            collapsed.push(line.clone());
        }
    }

    if !found {
        // Add blank line before if file is non-empty and doesn't end with one
        if !collapsed.is_empty() && collapsed.last().is_some_and(|l| !l.is_empty()) {
            collapsed.push(String::new());
        }
        collapsed.push(theme_line.to_string());
    }

    // Ensure trailing newline
    collapsed.push(String::new());

    tokio::fs::write(&config_path, collapsed.join("\n")).await?;
    Ok(())
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_theme_format() {
        let palette = ColorPalette::sample();
        let theme = generate_theme(&palette);

        // Verify Ghostty format (no quotes around hex values)
        assert!(theme.contains("background = #1B1B1B"));
        assert!(theme.contains("foreground = #FFFFFF"));
        assert!(theme.contains("cursor-color = #FFFFFF"));
        assert!(theme.contains("selection-foreground = #1B1B1B"));
        assert!(theme.contains("selection-background = #FFFFFF"));
        assert!(theme.contains("palette = 0=#3B3B3B"));
        assert!(theme.contains("palette = 15=#E8E8E8"));

        // Should have 21 lines (5 named + 16 palette)
        let non_empty: Vec<&str> = theme.lines().filter(|l| !l.is_empty()).collect();
        assert_eq!(non_empty.len(), 21);
    }

    #[test]
    fn test_no_font_settings() {
        let palette = ColorPalette::sample();
        let theme = generate_theme(&palette);

        assert!(!theme.contains("font-family"));
        assert!(!theme.contains("font-size"));
    }
}
