// SPDX-License-Identifier: GPL-3.0-only

//! Wallpaper configuration reading
//!
//! Reads COSMIC background settings and lists available wallpapers.

use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

/// Wallpaper configuration extracted from COSMIC
#[derive(Debug, Clone)]
#[allow(dead_code)] // Fields will be used as UI expands
pub struct WallpaperConfig {
    /// Current wallpaper source (path or directory)
    pub current_source: String,
    /// Rotation frequency in seconds
    pub rotation_frequency: u32,
    /// Scaling mode (Zoom, Fit, etc.)
    pub scaling_mode: String,
    /// Whether filtering by theme is enabled
    pub filter_by_theme: bool,
    /// Available theme wallpaper directories with their wallpaper counts
    pub available_themes: HashMap<String, ThemeWallpapers>,
}

/// Wallpapers for a specific theme
#[derive(Debug, Clone)]
#[allow(dead_code)] // Fields will be used when wallpaper selection UI is added
pub struct ThemeWallpapers {
    /// Theme name
    pub name: String,
    /// Directory path
    pub path: PathBuf,
    /// Number of wallpapers
    pub count: usize,
    /// List of wallpaper filenames
    pub wallpapers: Vec<String>,
}

impl Default for WallpaperConfig {
    fn default() -> Self {
        Self {
            current_source: String::new(),
            rotation_frequency: 600,
            scaling_mode: "Zoom".to_string(),
            filter_by_theme: true,
            available_themes: HashMap::new(),
        }
    }
}

impl WallpaperConfig {
    /// System wallpaper directory
    pub const SYSTEM_WALLPAPERS: &'static str = "/usr/share/backgrounds";

    /// COSMIC background config path
    pub fn config_path() -> PathBuf {
        directories::BaseDirs::new()
            .map(|dirs| dirs.config_dir().to_path_buf())
            .unwrap_or_else(|| PathBuf::from(".config"))
            .join("cosmic")
            .join("com.system76.CosmicBackground")
            .join("v1")
            .join("all")
    }

    /// Load wallpaper configuration
    pub fn load() -> Self {
        let mut config = Self::default();

        // Read COSMIC background config
        if let Ok(content) = fs::read_to_string(Self::config_path()) {
            config.parse_cosmic_config(&content);
        }

        // Scan available wallpapers
        config.scan_wallpapers();

        config
    }

    /// Parse COSMIC background RON config
    fn parse_cosmic_config(&mut self, content: &str) {
        // Simple parsing of RON format - extract key values
        for line in content.lines() {
            let line = line.trim();

            if line.starts_with("source:") {
                // Extract path from: source: Path("/path/to/wallpaper"),
                if let Some(start) = line.find('"') {
                    if let Some(end) = line.rfind('"') {
                        if start < end {
                            self.current_source = line[start + 1..end].to_string();
                        }
                    }
                }
            } else if line.starts_with("rotation_frequency:") {
                if let Some(value) = line.strip_prefix("rotation_frequency:") {
                    if let Ok(freq) = value.trim().trim_end_matches(',').parse() {
                        self.rotation_frequency = freq;
                    }
                }
            } else if line.starts_with("scaling_mode:") {
                if let Some(value) = line.strip_prefix("scaling_mode:") {
                    self.scaling_mode = value.trim().trim_end_matches(',').to_string();
                }
            } else if line.starts_with("filter_by_theme:") {
                if let Some(value) = line.strip_prefix("filter_by_theme:") {
                    self.filter_by_theme = value.trim().trim_end_matches(',') == "true";
                }
            }
        }
    }

    /// Scan system wallpaper directories
    fn scan_wallpapers(&mut self) {
        let wallpaper_dir = PathBuf::from(Self::SYSTEM_WALLPAPERS);

        if !wallpaper_dir.exists() {
            return;
        }

        if let Ok(entries) = fs::read_dir(&wallpaper_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                        let wallpapers = Self::list_wallpapers_in_dir(&path);
                        let count = wallpapers.len();

                        self.available_themes.insert(
                            name.to_string(),
                            ThemeWallpapers {
                                name: name.to_string(),
                                path,
                                count,
                                wallpapers,
                            },
                        );
                    }
                }
            }
        }
    }

    /// List wallpaper files in a directory
    fn list_wallpapers_in_dir(dir: &PathBuf) -> Vec<String> {
        let mut wallpapers = Vec::new();

        if let Ok(entries) = fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_file() {
                    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                        let ext_lower = ext.to_lowercase();
                        if matches!(
                            ext_lower.as_str(),
                            "png" | "jpg" | "jpeg" | "webp" | "gif" | "bmp"
                        ) {
                            if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                                wallpapers.push(name.to_string());
                            }
                        }
                    }
                }
            }
        }

        wallpapers.sort();
        wallpapers
    }

    /// Get total wallpaper count across all themes
    pub fn total_wallpaper_count(&self) -> usize {
        self.available_themes.values().map(|t| t.count).sum()
    }

    /// Get sorted list of theme names
    pub fn theme_names(&self) -> Vec<String> {
        let mut names: Vec<_> = self.available_themes.keys().cloned().collect();
        names.sort();
        names
    }

    /// Get the current wallpaper filename
    pub fn current_wallpaper_name(&self) -> String {
        if self.current_source.is_empty() {
            return "None".to_string();
        }

        PathBuf::from(&self.current_source)
            .file_name()
            .and_then(|n| n.to_str())
            .map(|s| s.to_string())
            .unwrap_or_else(|| "Unknown".to_string())
    }

    /// Get the current wallpaper theme (directory name)
    pub fn current_theme_name(&self) -> String {
        if self.current_source.is_empty() {
            return "None".to_string();
        }

        PathBuf::from(&self.current_source)
            .parent()
            .and_then(|p| p.file_name())
            .and_then(|n| n.to_str())
            .map(|s| s.to_string())
            .unwrap_or_else(|| "Unknown".to_string())
    }

    /// Format rotation frequency for display
    pub fn format_rotation(&self) -> String {
        if self.rotation_frequency == 0 {
            "Disabled".to_string()
        } else if self.rotation_frequency < 60 {
            format!("{} seconds", self.rotation_frequency)
        } else if self.rotation_frequency % 60 == 0 {
            let minutes = self.rotation_frequency / 60;
            if minutes == 1 {
                "1 minute".to_string()
            } else {
                format!("{} minutes", minutes)
            }
        } else {
            let minutes = self.rotation_frequency / 60;
            let secs = self.rotation_frequency % 60;
            format!("{}m {}s", minutes, secs)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_config() {
        let content = r#"
(
    output: "all",
    source: Path("/usr/share/backgrounds/gruvbox/1-gruvbox.png"),
    filter_by_theme: true,
    rotation_frequency: 300,
    scaling_mode: Fit,
)
"#;
        let mut config = WallpaperConfig::default();
        config.parse_cosmic_config(content);

        assert_eq!(
            config.current_source,
            "/usr/share/backgrounds/gruvbox/1-gruvbox.png"
        );
        assert_eq!(config.rotation_frequency, 300);
        assert_eq!(config.scaling_mode, "Fit");
        assert!(config.filter_by_theme);
    }

    #[test]
    fn test_format_rotation() {
        let mut config = WallpaperConfig::default();

        config.rotation_frequency = 0;
        assert_eq!(config.format_rotation(), "Disabled");

        config.rotation_frequency = 30;
        assert_eq!(config.format_rotation(), "30 seconds");

        config.rotation_frequency = 60;
        assert_eq!(config.format_rotation(), "1 minute");

        config.rotation_frequency = 600;
        assert_eq!(config.format_rotation(), "10 minutes");

        config.rotation_frequency = 90;
        assert_eq!(config.format_rotation(), "1m 30s");
    }
}
