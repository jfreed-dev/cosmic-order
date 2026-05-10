// SPDX-License-Identifier: GPL-3.0-only

//! lazygit theme generator
//!
//! Generates a lazygit `gui.theme` YAML block from the COSMIC palette
//! and updates the user's lazygit config using a comment-marker approach.

use std::fmt::Write;
use std::path::PathBuf;

use crate::colors::ColorPalette;
use crate::paths;

/// Comment marker for detecting our managed block
const MARKER: &str = "# COSMIC ORDER theme — auto-generated";

/// Generate the lazygit gui.theme YAML block
pub fn generate_theme_block(palette: &ColorPalette) -> String {
    let mut out = String::with_capacity(512);
    let _ = writeln!(out, "{MARKER}");
    out.push_str("gui:\n");
    out.push_str("  theme:\n");
    let _ = write!(
        out,
        "    activeBorderColor:\n      - \"{}\"\n      - bold\n",
        palette.accent
    );
    let _ = write!(
        out,
        "    inactiveBorderColor:\n      - \"{}\"\n",
        palette.colors[8]
    );
    let _ = write!(
        out,
        "    searchingActiveBorderColor:\n      - \"{}\"\n      - bold\n",
        palette.accent
    );
    let _ = write!(
        out,
        "    optionsTextColor:\n      - \"{}\"\n",
        palette.colors[4]
    );
    let _ = write!(
        out,
        "    selectedLineBgColor:\n      - \"{}\"\n",
        palette.colors[0]
    );
    let _ = write!(
        out,
        "    cherryPickedCommitFgColor:\n      - \"{}\"\n",
        palette.colors[4]
    );
    let _ = write!(
        out,
        "    cherryPickedCommitBgColor:\n      - \"{}\"\n",
        palette.colors[5]
    );
    let _ = write!(
        out,
        "    markedBaseCommitFgColor:\n      - \"{}\"\n",
        palette.colors[4]
    );
    let _ = write!(
        out,
        "    markedBaseCommitBgColor:\n      - \"{}\"\n",
        palette.colors[3]
    );
    let _ = write!(
        out,
        "    unstagedChangesColor:\n      - \"{}\"\n",
        palette.colors[1]
    );
    let _ = write!(
        out,
        "    defaultFgColor:\n      - \"{}\"\n",
        palette.foreground
    );

    out
}

/// Write theme to `~/.config/lazygit/config.yml`
///
/// Uses comment-marker approach to detect and replace our managed block.
pub async fn write_theme(palette: &ColorPalette) -> Result<PathBuf, std::io::Error> {
    let config_path = paths::lazygit_config();

    if let Some(parent) = config_path.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }

    let contents = match tokio::fs::read_to_string(&config_path).await {
        Ok(c) => c,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => String::new(),
        Err(e) => return Err(e),
    };

    let theme_block = generate_theme_block(palette);
    let new_contents = update_config(&contents, &theme_block);

    tokio::fs::write(&config_path, new_contents).await?;
    Ok(config_path)
}

/// Update config: replace existing COSMIC block or append at end
fn update_config(contents: &str, theme_block: &str) -> String {
    let mut lines: Vec<String> = Vec::new();
    let mut skip_block = false;
    let mut found_block = false;

    for line in contents.lines() {
        let trimmed = line.trim();

        // Detect our marker
        if trimmed == MARKER {
            skip_block = true;
            found_block = true;
            continue;
        }

        if skip_block {
            // Skip indented lines (part of our gui: theme: block)
            // Stop skipping when we hit a non-indented, non-empty line
            // that isn't a continuation of the YAML block
            if trimmed.is_empty() || line.starts_with(' ') || line.starts_with('\t') {
                continue;
            }
            // Also skip the `gui:` line that's part of our block
            if trimmed == "gui:" {
                continue;
            }
            skip_block = false;
        }

        lines.push(line.to_string());
    }

    // Append the new block
    if !found_block && lines.last().is_some_and(|l| !l.is_empty()) {
        lines.push(String::new());
    }

    // Trim trailing empty lines before appending
    while lines.last().is_some_and(std::string::String::is_empty) {
        lines.pop();
    }

    if !lines.is_empty() {
        lines.push(String::new());
    }
    lines.push(theme_block.trim_end().to_string());

    // Ensure trailing newline
    lines.push(String::new());
    lines.join("\n")
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_theme_block_format() {
        let palette = ColorPalette::sample();
        let block = generate_theme_block(&palette);

        assert!(block.starts_with(MARKER));
        assert!(block.contains("gui:"));
        assert!(block.contains("  theme:"));
        assert!(block.contains("activeBorderColor:"));
        assert!(block.contains("defaultFgColor:"));
    }

    #[test]
    fn test_generate_theme_block_colors() {
        let palette = ColorPalette::sample();
        let block = generate_theme_block(&palette);

        // Accent for active border
        assert!(block.contains("\"#63D1D6\""));
        // Bright black for inactive border
        assert!(block.contains("\"#5A5A5A\""));
        // Red for unstaged changes
        assert!(block.contains("\"#FF6B6B\""));
        // Blue for options text
        assert!(block.contains("\"#6B9FFF\""));
        // Foreground for default fg
        assert!(block.contains("\"#FFFFFF\""));
    }

    #[test]
    fn test_generate_theme_block_has_11_keys() {
        let palette = ColorPalette::sample();
        let block = generate_theme_block(&palette);

        let key_lines: Vec<&str> = block
            .lines()
            .filter(|l| {
                let t = l.trim();
                t.ends_with(':') && t != "gui:" && t != "theme:"
            })
            .collect();
        assert_eq!(key_lines.len(), 11);
    }

    #[test]
    fn test_update_config_fresh() {
        let block = "# COSMIC ORDER theme — auto-generated\ngui:\n  theme:\n    defaultFgColor:\n      - \"#FFFFFF\"";
        let result = update_config("", block);
        assert!(result.contains(MARKER));
        assert!(result.contains("defaultFgColor:"));
    }

    #[test]
    fn test_update_config_preserves_other_settings() {
        let existing = "notATheme:\n  someSetting: true\n\nother: value\n";
        let block = "# COSMIC ORDER theme — auto-generated\ngui:\n  theme:\n    defaultFgColor:\n      - \"#FFFFFF\"";
        let result = update_config(existing, block);
        assert!(result.contains("notATheme:"));
        assert!(result.contains("someSetting: true"));
        assert!(result.contains("other: value"));
        assert!(result.contains(MARKER));
    }

    #[test]
    fn test_update_config_replaces_old_block() {
        let existing = "someOther: true\n\n# COSMIC ORDER theme — auto-generated\ngui:\n  theme:\n    defaultFgColor:\n      - \"#000000\"\n\nanotherSetting: yes\n";
        let block = "# COSMIC ORDER theme — auto-generated\ngui:\n  theme:\n    defaultFgColor:\n      - \"#FFFFFF\"";
        let result = update_config(existing, block);

        assert!(result.contains("\"#FFFFFF\""));
        assert!(!result.contains("\"#000000\""));
        assert!(result.contains("someOther: true"));
        assert!(result.contains("anotherSetting: yes"));
    }
}
