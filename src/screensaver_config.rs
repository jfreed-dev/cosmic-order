// SPDX-License-Identifier: GPL-3.0-only

//! Screensaver configuration reading
//!
//! Reads the shell-style config from ~/.config/cosmic-screensaver/config

use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

use directories::BaseDirs;

/// Screensaver configuration
#[derive(Debug, Clone, Default)]
pub struct ScreensaverConfig {
    /// Whether screensaver is enabled
    pub enabled: bool,
    /// Idle timeout before screensaver starts (seconds)
    pub idle_timeout: u32,
    /// Lock timeout after screensaver (seconds, 0 to disable)
    pub lock_timeout: u32,
    /// DPMS (screen off) timeout (seconds, 0 to disable)
    pub dpms_timeout: u32,
    /// Animation frame rate
    pub frame_rate: u32,
    /// Effects to include (comma-separated)
    pub include_effects: String,
    /// Effects to exclude (comma-separated)
    pub exclude_effects: String,
    /// Fade in effect
    pub fade_in_effect: String,
    /// Fade out effect
    pub fade_out_effect: String,
    /// Show clock between effects
    pub show_clock: bool,
    /// Clock display duration (seconds)
    pub clock_duration: u32,
    /// Clock format string
    pub clock_format: String,
    /// Clock font
    pub clock_font: String,
    /// Logo file path
    pub logo_file: String,
    /// Disable on battery
    pub disable_on_battery: bool,
    /// Battery idle timeout (longer timeout when on battery)
    pub battery_idle_timeout: u32,
    /// Terminal emulator to use
    pub terminal: String,
}

impl ScreensaverConfig {
    /// Get the config file path
    pub fn config_path() -> PathBuf {
        BaseDirs::new()
            .map(|dirs| dirs.config_dir().to_path_buf())
            .unwrap_or_else(|| PathBuf::from(".config"))
            .join("cosmic-screensaver")
            .join("config")
    }

    /// Load configuration from file
    pub fn load() -> Result<Self, ConfigError> {
        let path = Self::config_path();

        if !path.exists() {
            return Ok(Self::default_config());
        }

        let content = fs::read_to_string(&path).map_err(|e| ConfigError::Read(e.to_string()))?;

        Self::parse(&content)
    }

    /// Parse configuration from string content
    fn parse(content: &str) -> Result<Self, ConfigError> {
        let mut values: HashMap<String, String> = HashMap::new();

        for line in content.lines() {
            let line = line.trim();

            // Skip empty lines and comments
            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            // Parse KEY="value" or KEY=value
            if let Some((key, value)) = line.split_once('=') {
                let key = key.trim().to_string();
                let value = value.trim().trim_matches('"').to_string();
                values.insert(key, value);
            }
        }

        Ok(Self {
            enabled: Self::parse_bool(&values, "ENABLED", true),
            idle_timeout: Self::parse_u32(&values, "IDLE_TIMEOUT", 300),
            lock_timeout: Self::parse_u32(&values, "LOCK_TIMEOUT", 600),
            dpms_timeout: Self::parse_u32(&values, "DPMS_TIMEOUT", 900),
            frame_rate: Self::parse_u32(&values, "FRAME_RATE", 60),
            include_effects: values.get("INCLUDE_EFFECTS").cloned().unwrap_or_default(),
            exclude_effects: values
                .get("EXCLUDE_EFFECTS")
                .cloned()
                .unwrap_or_else(|| "dev_worm".to_string()),
            fade_in_effect: values.get("FADE_IN_EFFECT").cloned().unwrap_or_default(),
            fade_out_effect: values.get("FADE_OUT_EFFECT").cloned().unwrap_or_default(),
            show_clock: Self::parse_bool(&values, "SHOW_CLOCK", false),
            clock_duration: Self::parse_u32(&values, "CLOCK_DURATION", 5),
            clock_format: values
                .get("CLOCK_FORMAT")
                .cloned()
                .unwrap_or_else(|| "%H:%M".to_string()),
            clock_font: values.get("CLOCK_FONT").cloned().unwrap_or_default(),
            logo_file: values.get("LOGO_FILE").cloned().unwrap_or_default(),
            disable_on_battery: Self::parse_bool(&values, "DISABLE_ON_BATTERY", false),
            battery_idle_timeout: Self::parse_u32(&values, "BATTERY_IDLE_TIMEOUT", 600),
            terminal: values
                .get("TERMINAL")
                .cloned()
                .unwrap_or_else(|| "ghostty".to_string()),
        })
    }

    /// Parse a boolean value from the config
    fn parse_bool(values: &HashMap<String, String>, key: &str, default: bool) -> bool {
        values
            .get(key)
            .map(|v| v.eq_ignore_ascii_case("true") || v == "1")
            .unwrap_or(default)
    }

    /// Parse a u32 value from the config
    fn parse_u32(values: &HashMap<String, String>, key: &str, default: u32) -> u32 {
        values
            .get(key)
            .and_then(|v| v.parse().ok())
            .unwrap_or(default)
    }

    /// Get default configuration values
    fn default_config() -> Self {
        Self {
            enabled: true,
            idle_timeout: 300,
            lock_timeout: 600,
            dpms_timeout: 900,
            frame_rate: 60,
            include_effects: String::new(),
            exclude_effects: "dev_worm".to_string(),
            fade_in_effect: String::new(),
            fade_out_effect: String::new(),
            show_clock: false,
            clock_duration: 5,
            clock_format: "%H:%M".to_string(),
            clock_font: String::new(),
            logo_file: String::new(),
            disable_on_battery: false,
            battery_idle_timeout: 600,
            terminal: "ghostty".to_string(),
        }
    }

    /// Serialize config to shell KEY="value" format matching screensaver-ctl.sh
    pub fn serialize(&self) -> String {
        let bool_str = |b: bool| if b { "true" } else { "false" };
        format!(
            r#"ENABLED="{}"
IDLE_TIMEOUT="{}"
LOCK_TIMEOUT="{}"
DPMS_TIMEOUT="{}"
FRAME_RATE="{}"
INCLUDE_EFFECTS="{}"
EXCLUDE_EFFECTS="{}"
FADE_IN_EFFECT="{}"
FADE_OUT_EFFECT="{}"
SHOW_CLOCK="{}"
CLOCK_DURATION="{}"
CLOCK_FORMAT="{}"
CLOCK_FONT="{}"
LOGO_FILE="{}"
DISABLE_ON_BATTERY="{}"
BATTERY_IDLE_TIMEOUT="{}"
TERMINAL="{}"
"#,
            bool_str(self.enabled),
            self.idle_timeout,
            self.lock_timeout,
            self.dpms_timeout,
            self.frame_rate,
            self.include_effects,
            self.exclude_effects,
            self.fade_in_effect,
            self.fade_out_effect,
            bool_str(self.show_clock),
            self.clock_duration,
            self.clock_format,
            self.clock_font,
            self.logo_file,
            bool_str(self.disable_on_battery),
            self.battery_idle_timeout,
            self.terminal,
        )
    }

    /// Save configuration to file
    pub fn save(&self) -> Result<(), ConfigError> {
        let path = Self::config_path();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| ConfigError::Write(format!("create dirs: {e}")))?;
        }
        fs::write(&path, self.serialize())
            .map_err(|e| ConfigError::Write(format!("write file: {e}")))?;
        Ok(())
    }

    /// Path to the screensaver-ctl script
    pub fn ctl_path() -> PathBuf {
        BaseDirs::new()
            .map(|dirs| dirs.home_dir().join(".local/bin/screensaver-ctl"))
            .unwrap_or_else(|| PathBuf::from("screensaver-ctl"))
    }

    /// Format timeout value for display (used in tests, may be useful in future UI)
    #[allow(dead_code)]
    pub fn format_timeout(seconds: u32) -> String {
        if seconds == 0 {
            "Disabled".to_string()
        } else if seconds < 60 {
            format!("{} seconds", seconds)
        } else if seconds % 60 == 0 {
            let minutes = seconds / 60;
            if minutes == 1 {
                "1 minute".to_string()
            } else {
                format!("{} minutes", minutes)
            }
        } else {
            let minutes = seconds / 60;
            let secs = seconds % 60;
            format!("{}m {}s", minutes, secs)
        }
    }

    /// Get the logo name from the file path
    pub fn logo_name(&self) -> String {
        if self.logo_file.is_empty() {
            return "None".to_string();
        }

        PathBuf::from(&self.logo_file)
            .file_stem()
            .and_then(|s| s.to_str())
            .map(|s| s.replace(['-', '_'], " "))
            .unwrap_or_else(|| "Custom".to_string())
    }
}

/// Configuration errors
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("Failed to read configuration: {0}")]
    Read(String),
    #[error("Failed to write configuration: {0}")]
    Write(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_config() {
        let content = r#"
# Test config
ENABLED="true"
IDLE_TIMEOUT="300"
LOCK_TIMEOUT="600"
SHOW_CLOCK="false"
LOGO_FILE="/home/user/.config/cosmic-screensaver/logo.txt"
"#;

        let config = ScreensaverConfig::parse(content).unwrap();
        assert!(config.enabled);
        assert_eq!(config.idle_timeout, 300);
        assert_eq!(config.lock_timeout, 600);
        assert!(!config.show_clock);
        assert!(config.logo_file.contains("logo.txt"));
    }

    #[test]
    fn test_serialize_roundtrip() {
        let config = ScreensaverConfig {
            enabled: true,
            idle_timeout: 300,
            lock_timeout: 600,
            dpms_timeout: 900,
            frame_rate: 120,
            include_effects: "matrix,rain".to_string(),
            exclude_effects: "dev_worm".to_string(),
            fade_in_effect: "fade".to_string(),
            fade_out_effect: "slide".to_string(),
            show_clock: true,
            clock_duration: 10,
            clock_format: "%H:%M:%S".to_string(),
            clock_font: "monospace".to_string(),
            logo_file: "/home/user/logo.txt".to_string(),
            disable_on_battery: true,
            battery_idle_timeout: 120,
            terminal: "cosmic-term".to_string(),
        };

        let serialized = config.serialize();
        let parsed = ScreensaverConfig::parse(&serialized).unwrap();

        assert_eq!(parsed.enabled, config.enabled);
        assert_eq!(parsed.idle_timeout, config.idle_timeout);
        assert_eq!(parsed.lock_timeout, config.lock_timeout);
        assert_eq!(parsed.dpms_timeout, config.dpms_timeout);
        assert_eq!(parsed.frame_rate, config.frame_rate);
        assert_eq!(parsed.include_effects, config.include_effects);
        assert_eq!(parsed.exclude_effects, config.exclude_effects);
        assert_eq!(parsed.fade_in_effect, config.fade_in_effect);
        assert_eq!(parsed.fade_out_effect, config.fade_out_effect);
        assert_eq!(parsed.show_clock, config.show_clock);
        assert_eq!(parsed.clock_duration, config.clock_duration);
        assert_eq!(parsed.clock_format, config.clock_format);
        assert_eq!(parsed.clock_font, config.clock_font);
        assert_eq!(parsed.logo_file, config.logo_file);
        assert_eq!(parsed.disable_on_battery, config.disable_on_battery);
        assert_eq!(parsed.battery_idle_timeout, config.battery_idle_timeout);
        assert_eq!(parsed.terminal, config.terminal);
    }

    #[test]
    fn test_format_timeout() {
        assert_eq!(ScreensaverConfig::format_timeout(0), "Disabled");
        assert_eq!(ScreensaverConfig::format_timeout(30), "30 seconds");
        assert_eq!(ScreensaverConfig::format_timeout(60), "1 minute");
        assert_eq!(ScreensaverConfig::format_timeout(300), "5 minutes");
        assert_eq!(ScreensaverConfig::format_timeout(90), "1m 30s");
    }
}
