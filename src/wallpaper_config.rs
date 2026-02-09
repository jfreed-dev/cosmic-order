// SPDX-License-Identifier: GPL-3.0-only

//! Wallpaper configuration reading and writing
//!
//! Reads and modifies COSMIC background settings and lists available wallpapers.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// On-disk RON format for COSMIC background config
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CosmicBgEntry {
    pub output: String,
    pub source: BgSource,
    pub filter_by_theme: bool,
    pub rotation_frequency: u32,
    pub filter_method: FilterMethod,
    pub scaling_mode: ScalingMode,
    pub sampling_method: SamplingMethod,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BgSource {
    Path(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum FilterMethod {
    Lanczos,
    Linear,
    Nearest,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ScalingMode {
    Zoom,
    Fit,
    Stretch,
    Center,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum SamplingMethod {
    Random,
    Alphanumeric,
}

impl ScalingMode {
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Zoom => "Zoom",
            Self::Fit => "Fit",
            Self::Stretch => "Stretch",
            Self::Center => "Center",
        }
    }

    pub const fn all() -> &'static [Self] {
        &[Self::Zoom, Self::Fit, Self::Stretch, Self::Center]
    }
}

impl std::fmt::Display for ScalingMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Wallpaper operation errors
#[derive(Debug, thiserror::Error)]
pub enum WallpaperError {
    #[error("Failed to write wallpaper config: {0}")]
    ConfigWrite(String),
    #[error("Failed to serialize wallpaper config: {0}")]
    SerializeError(String),
    #[error("Failed to create directory: {0}")]
    CreateDir(String),
}

/// Wallpaper configuration extracted from COSMIC
#[derive(Debug, Clone)]
pub struct WallpaperConfig {
    /// Current wallpaper source (path or directory)
    pub current_source: String,
    /// Rotation frequency in seconds
    pub rotation_frequency: u32,
    /// Scaling mode
    pub scaling_mode: ScalingMode,
    /// Filter method
    pub filter_method: FilterMethod,
    /// Sampling method
    pub sampling_method: SamplingMethod,
    /// Whether filtering by theme is enabled
    pub filter_by_theme: bool,
    /// Whether rotation is enabled (frequency > 0)
    pub rotation_enabled: bool,
    /// Available theme wallpaper directories with their wallpaper counts
    pub available_themes: HashMap<String, ThemeWallpapers>,
}

/// Wallpapers for a specific theme
#[derive(Debug, Clone)]
pub struct ThemeWallpapers {
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
            scaling_mode: ScalingMode::Zoom,
            filter_method: FilterMethod::Lanczos,
            sampling_method: SamplingMethod::Random,
            filter_by_theme: true,
            rotation_enabled: true,
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
            .map_or_else(
                || PathBuf::from(".config"),
                |dirs| dirs.config_dir().to_path_buf(),
            )
            .join("cosmic")
            .join("com.system76.CosmicBackground")
            .join("v1")
            .join("all")
    }

    /// User wallpapers directory for imported files
    pub fn user_wallpapers_dir() -> PathBuf {
        directories::BaseDirs::new()
            .map_or_else(
                || PathBuf::from(".local/share"),
                |dirs| dirs.data_local_dir().to_path_buf(),
            )
            .join("backgrounds")
            .join("custom")
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

    /// Parse COSMIC background RON config — tries proper RON deserialization first,
    /// falls back to manual line parsing if that fails
    fn parse_cosmic_config(&mut self, content: &str) {
        if let Ok(entry) = ron::from_str::<CosmicBgEntry>(content) {
            match &entry.source {
                BgSource::Path(p) => self.current_source = p.clone(),
            }
            self.filter_by_theme = entry.filter_by_theme;
            self.rotation_frequency = entry.rotation_frequency;
            self.rotation_enabled = entry.rotation_frequency > 0;
            self.filter_method = entry.filter_method;
            self.scaling_mode = entry.scaling_mode;
            self.sampling_method = entry.sampling_method;
            return;
        }

        // Fallback: manual line parsing for older/non-standard configs
        tracing::debug!("RON parse failed, falling back to manual parsing");
        self.parse_cosmic_config_fallback(content);
    }

    /// Fallback manual line parser for non-standard config formats
    fn parse_cosmic_config_fallback(&mut self, content: &str) {
        for line in content.lines() {
            let line = line.trim();

            if line.starts_with("source:") {
                if let Some(start) = line.find('"')
                    && let Some(end) = line.rfind('"')
                    && start < end
                {
                    self.current_source = line[start + 1..end].to_string();
                }
            } else if line.starts_with("rotation_frequency:") {
                if let Some(value) = line.strip_prefix("rotation_frequency:")
                    && let Ok(freq) = value.trim().trim_end_matches(',').parse()
                {
                    self.rotation_frequency = freq;
                    self.rotation_enabled = self.rotation_frequency > 0;
                }
            } else if line.starts_with("scaling_mode:") {
                if let Some(value) = line.strip_prefix("scaling_mode:") {
                    let val = value.trim().trim_end_matches(',');
                    self.scaling_mode = match val {
                        "Fit" => ScalingMode::Fit,
                        "Stretch" => ScalingMode::Stretch,
                        "Center" => ScalingMode::Center,
                        _ => ScalingMode::Zoom,
                    };
                }
            } else if line.starts_with("filter_by_theme:")
                && let Some(value) = line.strip_prefix("filter_by_theme:")
            {
                self.filter_by_theme = value.trim().trim_end_matches(',') == "true";
            }
        }
    }

    /// Build a `CosmicBgEntry` from the current config state
    fn to_bg_entry(&self) -> CosmicBgEntry {
        CosmicBgEntry {
            output: "all".to_string(),
            source: BgSource::Path(self.current_source.clone()),
            filter_by_theme: self.filter_by_theme,
            rotation_frequency: if self.rotation_enabled {
                self.rotation_frequency
            } else {
                0
            },
            filter_method: self.filter_method.clone(),
            scaling_mode: self.scaling_mode.clone(),
            sampling_method: self.sampling_method.clone(),
        }
    }

    /// Save current config state to disk
    pub fn save(&self) -> Result<(), WallpaperError> {
        let entry = self.to_bg_entry();
        let pretty = ron::ser::PrettyConfig::default();
        let serialized = ron::ser::to_string_pretty(&entry, pretty)
            .map_err(|e| WallpaperError::SerializeError(e.to_string()))?;

        let config_path = Self::config_path();
        if let Some(parent) = config_path.parent() {
            fs::create_dir_all(parent).map_err(|e| WallpaperError::CreateDir(e.to_string()))?;
        }

        fs::write(&config_path, serialized)
            .map_err(|e| WallpaperError::ConfigWrite(e.to_string()))?;

        Ok(())
    }

    /// Scan system and user wallpaper directories
    fn scan_wallpapers(&mut self) {
        // Scan system wallpapers
        let wallpaper_dir = PathBuf::from(Self::SYSTEM_WALLPAPERS);
        if wallpaper_dir.exists() {
            self.scan_directory(&wallpaper_dir);
        }

        // Scan user wallpapers directory
        let user_dir = Self::user_wallpapers_dir();
        if user_dir.exists() {
            let wallpapers = Self::list_wallpapers_in_dir(&user_dir);
            let count = wallpapers.len();
            if count > 0 {
                self.available_themes.insert(
                    "custom".to_string(),
                    ThemeWallpapers {
                        path: user_dir,
                        count,
                        wallpapers,
                    },
                );
            }
        }
    }

    /// Scan a directory for theme subdirectories containing wallpapers
    fn scan_directory(&mut self, dir: &Path) {
        if let Ok(entries) = fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir()
                    && let Some(name) = path.file_name().and_then(|n| n.to_str())
                {
                    let wallpapers = Self::list_wallpapers_in_dir(&path);
                    let count = wallpapers.len();

                    self.available_themes.insert(
                        name.to_string(),
                        ThemeWallpapers {
                            path,
                            count,
                            wallpapers,
                        },
                    );
                }
            }
        }
    }

    /// List wallpaper files in a directory
    fn list_wallpapers_in_dir(dir: &Path) -> Vec<String> {
        let mut wallpapers = Vec::new();

        if let Ok(entries) = fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_file()
                    && let Some(ext) = path.extension().and_then(|e| e.to_str())
                {
                    let ext_lower = ext.to_lowercase();
                    if matches!(
                        ext_lower.as_str(),
                        "png" | "jpg" | "jpeg" | "webp" | "gif" | "bmp"
                    ) && let Some(name) = path.file_name().and_then(|n| n.to_str())
                    {
                        wallpapers.push(name.to_string());
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
            .map_or_else(|| "Unknown".to_string(), std::string::ToString::to_string)
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
            .map_or_else(|| "Unknown".to_string(), std::string::ToString::to_string)
    }
}

/// Thumbnail cache for wallpaper grid performance.
///
/// Generates small thumbnails in a cache directory so the grid doesn't
/// load full 5K images for 160px cards. Failures are cached as empty
/// marker files to avoid retrying on every frame.
pub struct ThumbnailCache {
    /// Cache directory path (public for background task access)
    pub cache_dir: PathBuf,
}

impl ThumbnailCache {
    const THUMB_WIDTH: u32 = 160;
    const THUMB_HEIGHT: u32 = 100;
    const PREVIEW_WIDTH: u32 = 640;
    const PREVIEW_HEIGHT: u32 = 400;

    pub fn new() -> Self {
        let cache_dir = directories::BaseDirs::new()
            .map_or_else(
                || PathBuf::from(".cache"),
                |dirs| dirs.cache_dir().to_path_buf(),
            )
            .join("cosmic-order")
            .join("thumbnails");
        Self { cache_dir }
    }

    /// Check if a thumbnail exists on disk (no I/O generation).
    /// Returns `Some(thumb_path)` if cached, `None` if not yet generated.
    pub fn get_cached(&self, source_path: &str) -> Option<PathBuf> {
        Self::check_cached_at(&self.thumb_path_for(source_path))
    }

    /// Check if a preview-size thumbnail exists on disk (no I/O generation).
    /// Returns `Some(preview_path)` if cached, `None` if not yet generated.
    pub fn get_cached_preview(&self, source_path: &str) -> Option<PathBuf> {
        Self::check_cached_at(&self.preview_path_for(source_path))
    }

    /// Check if a cached file exists and is not a failure marker.
    fn check_cached_at(path: &Path) -> Option<PathBuf> {
        if path.exists() {
            if fs::metadata(path).map(|m| m.len() == 0).unwrap_or(false) {
                return None;
            }
            return Some(path.to_path_buf());
        }
        None
    }

    /// Compute the cache key for a source image.
    fn cache_key_for(source_path: &str) -> String {
        let source = Path::new(source_path);
        source
            .parent()
            .and_then(|p| p.file_name())
            .and_then(|n| n.to_str())
            .map_or_else(
                || {
                    source
                        .file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("unknown")
                        .to_string()
                },
                |parent| {
                    let fname = source
                        .file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("unknown");
                    format!("{parent}__{fname}")
                },
            )
    }

    /// Compute the grid thumbnail path for a source image.
    fn thumb_path_for(&self, source_path: &str) -> PathBuf {
        self.cache_dir.join(Self::cache_key_for(source_path))
    }

    /// Compute the preview thumbnail path for a source image.
    fn preview_path_for(&self, source_path: &str) -> PathBuf {
        self.cache_dir
            .join("preview")
            .join(Self::cache_key_for(source_path))
    }

    /// Generate thumbnails for a batch of source paths (blocking I/O).
    /// Produces both grid (160x100) and preview (640x400) thumbnails.
    /// Returns the number of source images processed.
    pub fn generate_batch(&self, source_paths: &[String]) -> usize {
        let preview_dir = self.cache_dir.join("preview");
        let mut count = 0;
        for source_path in source_paths {
            if self.generate_both_tiers(source_path, &preview_dir) {
                count += 1;
            }
        }
        count
    }

    /// Generate grid + preview thumbnails for a single source image.
    /// Returns `true` if at least one tier was generated.
    fn generate_both_tiers(&self, source_path: &str, preview_dir: &Path) -> bool {
        let thumb_path = self.thumb_path_for(source_path);
        let preview_path = self.preview_path_for(source_path);
        let need_thumb = !thumb_path.exists();
        let need_preview = !preview_path.exists();
        if !need_thumb && !need_preview {
            return false;
        }

        let source = Path::new(source_path);
        let img = match image::open(source) {
            Ok(img) => img,
            Err(e) => {
                tracing::warn!("Thumbnail generation failed for {source_path}: {e}");
                Self::write_failure_markers(
                    need_thumb,
                    &thumb_path,
                    &self.cache_dir,
                    need_preview,
                    &preview_path,
                    preview_dir,
                );
                return false;
            }
        };

        if need_thumb {
            Self::save_or_mark(
                &img,
                &thumb_path,
                &self.cache_dir,
                Self::THUMB_WIDTH,
                Self::THUMB_HEIGHT,
                source_path,
                "Grid",
            );
        }
        if need_preview {
            Self::save_or_mark(
                &img,
                &preview_path,
                preview_dir,
                Self::PREVIEW_WIDTH,
                Self::PREVIEW_HEIGHT,
                source_path,
                "Preview",
            );
        }
        true
    }

    /// Write 0-byte failure markers for tiers that need them.
    fn write_failure_markers(
        need_thumb: bool,
        thumb_path: &Path,
        thumb_dir: &Path,
        need_preview: bool,
        preview_path: &Path,
        preview_dir: &Path,
    ) {
        if need_thumb {
            let _ = fs::create_dir_all(thumb_dir);
            let _ = fs::write(thumb_path, b"");
        }
        if need_preview {
            let _ = fs::create_dir_all(preview_dir);
            let _ = fs::write(preview_path, b"");
        }
    }

    /// Save a thumbnail or write a failure marker on error.
    fn save_or_mark(
        img: &image::DynamicImage,
        dest: &Path,
        dest_dir: &Path,
        width: u32,
        height: u32,
        source_path: &str,
        label: &str,
    ) {
        if let Err(e) = Self::save_thumbnail(img, dest, dest_dir, width, height) {
            tracing::warn!("{label} thumbnail save failed for {source_path}: {e}");
            let _ = fs::write(dest, b"");
        }
    }

    fn save_thumbnail(
        img: &image::DynamicImage,
        dest: &Path,
        dest_dir: &Path,
        width: u32,
        height: u32,
    ) -> Result<(), String> {
        fs::create_dir_all(dest_dir).map_err(|e| format!("Create cache dir: {e}"))?;
        let thumb = img.thumbnail(width, height);
        thumb
            .save(dest)
            .map_err(|e| format!("Save thumbnail: {e}"))?;
        Ok(())
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_config_ron() {
        let content = r#"(
    output: "all",
    source: Path("/usr/share/backgrounds/gruvbox/1-gruvbox.png"),
    filter_by_theme: true,
    rotation_frequency: 300,
    filter_method: Lanczos,
    scaling_mode: Fit,
    sampling_method: Random,
)"#;
        let mut config = WallpaperConfig::default();
        config.parse_cosmic_config(content);

        assert_eq!(
            config.current_source,
            "/usr/share/backgrounds/gruvbox/1-gruvbox.png"
        );
        assert_eq!(config.rotation_frequency, 300);
        assert_eq!(config.scaling_mode, ScalingMode::Fit);
        assert_eq!(config.filter_method, FilterMethod::Lanczos);
        assert_eq!(config.sampling_method, SamplingMethod::Random);
        assert!(config.filter_by_theme);
        assert!(config.rotation_enabled);
    }

    #[test]
    fn test_parse_config_fallback() {
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
        assert!(config.filter_by_theme);
    }

    #[test]
    fn test_ron_round_trip() {
        let entry = CosmicBgEntry {
            output: "all".to_string(),
            source: BgSource::Path("/usr/share/backgrounds/ethereal/1.jpg".to_string()),
            filter_by_theme: true,
            rotation_frequency: 600,
            filter_method: FilterMethod::Lanczos,
            scaling_mode: ScalingMode::Zoom,
            sampling_method: SamplingMethod::Random,
        };

        let pretty = ron::ser::PrettyConfig::default();
        let serialized = ron::ser::to_string_pretty(&entry, pretty).expect("Failed to serialize");
        let deserialized: CosmicBgEntry =
            ron::from_str(&serialized).expect("Failed to deserialize");

        assert_eq!(deserialized.output, "all");
        match &deserialized.source {
            BgSource::Path(p) => {
                assert_eq!(p, "/usr/share/backgrounds/ethereal/1.jpg");
            }
        }
        assert!(deserialized.filter_by_theme);
        assert_eq!(deserialized.rotation_frequency, 600);
        assert_eq!(deserialized.filter_method, FilterMethod::Lanczos);
        assert_eq!(deserialized.scaling_mode, ScalingMode::Zoom);
        assert_eq!(deserialized.sampling_method, SamplingMethod::Random);
    }

    #[test]
    fn test_scaling_mode_display() {
        assert_eq!(ScalingMode::Zoom.as_str(), "Zoom");
        assert_eq!(ScalingMode::Fit.as_str(), "Fit");
        assert_eq!(ScalingMode::Stretch.as_str(), "Stretch");
        assert_eq!(ScalingMode::Center.as_str(), "Center");
    }

    #[test]
    fn test_to_bg_entry() {
        let config = WallpaperConfig {
            current_source: "/test/path.jpg".to_string(),
            rotation_enabled: false,
            rotation_frequency: 600,
            ..Default::default()
        };

        let entry = config.to_bg_entry();
        assert_eq!(entry.rotation_frequency, 0); // Disabled overrides value
        match &entry.source {
            BgSource::Path(p) => assert_eq!(p, "/test/path.jpg"),
        }
    }
}
