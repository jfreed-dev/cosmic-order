// SPDX-License-Identifier: GPL-3.0-only

//! Ghostty terminal theme generator
//!
//! Generates a Ghostty theme file from the COSMIC color palette
//! and activates it in the user's Ghostty config.

use std::path::PathBuf;

use crate::colors::ColorPalette;

/// Generate Ghostty theme file content from a color palette
pub fn generate_theme(palette: &ColorPalette) -> String {
    let mut out = String::with_capacity(512);
    out.push_str(&format!("background = {}\n", palette.background));
    out.push_str(&format!("foreground = {}\n", palette.foreground));
    out.push_str(&format!("cursor-color = {}\n", palette.cursor));
    out.push_str(&format!(
        "selection-foreground = {}\n",
        palette.selection_foreground
    ));
    out.push_str(&format!(
        "selection-background = {}\n",
        palette.selection_background
    ));

    for (i, color) in palette.colors.iter().enumerate() {
        out.push_str(&format!("palette = {i}={color}\n"));
    }

    out
}

/// Write the generated theme to `~/.config/ghostty/themes/cosmic-synced`
pub async fn write_theme(palette: &ColorPalette) -> Result<PathBuf, std::io::Error> {
    let themes_dir = ghostty_themes_dir();
    tokio::fs::create_dir_all(&themes_dir).await?;
    let path = themes_dir.join("cosmic-synced");
    tokio::fs::write(&path, generate_theme(palette)).await?;
    Ok(path)
}

/// Activate the `cosmic-synced` theme in `~/.config/ghostty/config`
///
/// If a `theme =` line exists, replace it. Otherwise append one.
pub async fn activate_theme() -> Result<(), std::io::Error> {
    let config_path = ghostty_config_path();

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
        if line.trim_start().starts_with("theme") && line.contains('=') {
            new_lines.push(theme_line.to_string());
            found = true;
        } else {
            new_lines.push(line.to_string());
        }
    }

    if !found {
        // Add blank line before if file is non-empty and doesn't end with one
        if !new_lines.is_empty() && new_lines.last().is_some_and(|l| !l.is_empty()) {
            new_lines.push(String::new());
        }
        new_lines.push(theme_line.to_string());
    }

    // Ensure trailing newline
    new_lines.push(String::new());

    tokio::fs::write(&config_path, new_lines.join("\n")).await?;
    Ok(())
}

fn ghostty_config_path() -> PathBuf {
    directories::BaseDirs::new()
        .map(|d| d.config_dir().join("ghostty").join("config"))
        .unwrap_or_else(|| {
            let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
            PathBuf::from(home).join(".config/ghostty/config")
        })
}

fn ghostty_themes_dir() -> PathBuf {
    directories::BaseDirs::new()
        .map(|d| d.config_dir().join("ghostty").join("themes"))
        .unwrap_or_else(|| {
            let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
            PathBuf::from(home).join(".config/ghostty/themes")
        })
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    fn sample_palette() -> ColorPalette {
        ColorPalette {
            accent: "#63D1D6".to_string(),
            cursor: "#FFFFFF".to_string(),
            foreground: "#FFFFFF".to_string(),
            background: "#1B1B1B".to_string(),
            selection_foreground: "#1B1B1B".to_string(),
            selection_background: "#FFFFFF".to_string(),
            colors: [
                "#3B3B3B".to_string(),
                "#FF6B6B".to_string(),
                "#87D687".to_string(),
                "#FFD93D".to_string(),
                "#6B9FFF".to_string(),
                "#B87DFF".to_string(),
                "#63D1D6".to_string(),
                "#D4D4D4".to_string(),
                "#5A5A5A".to_string(),
                "#FF6B6B".to_string(),
                "#87D687".to_string(),
                "#FFD93D".to_string(),
                "#6B9FFF".to_string(),
                "#B87DFF".to_string(),
                "#63D1D6".to_string(),
                "#E8E8E8".to_string(),
            ],
        }
    }

    #[test]
    fn test_generate_theme_format() {
        let palette = sample_palette();
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
        let palette = sample_palette();
        let theme = generate_theme(&palette);

        assert!(!theme.contains("font-family"));
        assert!(!theme.contains("font-size"));
    }
}
