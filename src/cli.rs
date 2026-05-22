// SPDX-License-Identifier: GPL-3.0-only

//! CLI interface for scripting theme sync, color extraction, and theme switching.

use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Parser, Subcommand};
use serde::Serialize;

use crate::colors::{self, ColorPalette};
use crate::fl;
use crate::hooks;
use crate::theme_config::ThemeConfig;
use crate::tool_sync::{self, ToolSyncConfig};

/// COSMIC ORDER — OMARCHY-inspired workflow and aesthetics for COSMIC Desktop
#[derive(Parser)]
#[command(name = "cosmic-order", version, about)]
pub struct Cli {
    /// Open the GUI directly to a page instead of the last-used one
    #[arg(long, value_enum)]
    pub page: Option<crate::pages::PageId>,

    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    /// Sync COSMIC theme colors to all enabled tools
    Sync {
        /// Send reload signals to running applications after sync
        #[arg(long)]
        reload: bool,
        /// Output results as JSON
        #[arg(long)]
        json: bool,
    },
    /// Extract color palette from the current COSMIC theme
    Colors {
        #[command(subcommand)]
        action: Option<ColorsAction>,
        /// Output as JSON instead of TOML
        #[arg(long)]
        json: bool,
    },
    /// View and modify COSMIC theme settings
    Theme {
        #[command(subcommand)]
        action: ThemeAction,
    },
    /// Run user-defined hooks with the current palette
    Hooks {
        #[command(subcommand)]
        action: HooksAction,
    },
    /// Manage wallpapers
    Wallpaper {
        #[command(subcommand)]
        action: WallpaperAction,
    },
}

#[derive(Debug, Subcommand)]
pub enum ColorsAction {
    /// Save colors.toml to disk (default: ~/.config/cosmic-order/colors.toml)
    Save {
        /// Custom output path
        path: Option<PathBuf>,
    },
}

#[derive(Debug, Subcommand)]
pub enum ThemeAction {
    /// Show current theme information
    Info {
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
    /// Switch to dark mode
    Dark,
    /// Switch to light mode
    Light,
    /// Set the accent color (e.g. '#FF5733')
    SetAccent {
        /// Hex color value (e.g. '#FF5733')
        hex: String,
    },
    /// Export current theme to a .ron file
    Export {
        /// Output file path
        path: PathBuf,
    },
    /// Import a .ron theme file
    Import {
        /// Input file path
        path: PathBuf,
    },
}

#[derive(Debug, Subcommand)]
pub enum HooksAction {
    /// Run all hooks with the current color palette
    Run {
        /// Output results as JSON
        #[arg(long)]
        json: bool,
    },
}

#[derive(Debug, Subcommand)]
pub enum WallpaperAction {
    /// Download a wallpaper from a URL
    Add {
        /// Image URL to download
        url: String,
        /// Output as JSON
        #[arg(long)]
        json: bool,
        /// Set the downloaded image as the active COSMIC wallpaper
        #[arg(long)]
        set: bool,
    },
}

// --- JSON output structs ---

#[derive(Serialize)]
struct SyncOutput {
    colors_path: String,
    tools_synced: Vec<String>,
    hooks: Option<HooksOutput>,
    apps_reloaded: Vec<String>,
    /// Tools that have no live-reload mechanism; each entry tells the
    /// user what manual step is needed (e.g. re-source shell, restart).
    apps_manual: Vec<String>,
}

#[derive(Serialize)]
#[allow(clippy::struct_field_names)] // Mirrors HookResults field names for JSON output
struct HooksOutput {
    hooks_run: u32,
    hooks_succeeded: u32,
    hooks_failed: u32,
    hooks_timed_out: u32,
}

#[derive(Serialize)]
struct ThemeInfoOutput {
    name: String,
    mode: String,
    accent_color: String,
}

#[derive(Serialize)]
struct ColorsOutput {
    accent: String,
    cursor: String,
    foreground: String,
    background: String,
    selection_foreground: String,
    selection_background: String,
    colors: Vec<String>,
}

impl From<&ColorPalette> for ColorsOutput {
    fn from(p: &ColorPalette) -> Self {
        Self {
            accent: p.accent.clone(),
            cursor: p.cursor.clone(),
            foreground: p.foreground.clone(),
            background: p.background.clone(),
            selection_foreground: p.selection_foreground.clone(),
            selection_background: p.selection_background.clone(),
            colors: p.colors.to_vec(),
        }
    }
}

impl From<&hooks::HookResults> for HooksOutput {
    fn from(r: &hooks::HookResults) -> Self {
        Self {
            hooks_run: r.hooks_run,
            hooks_succeeded: r.hooks_succeeded,
            hooks_failed: r.hooks_failed,
            hooks_timed_out: r.hooks_timed_out,
        }
    }
}

/// Run a CLI command. Returns the process exit code.
pub fn run(command: Commands) -> ExitCode {
    let rt = match tokio::runtime::Runtime::new() {
        Ok(rt) => rt,
        Err(e) => {
            eprintln!("{}: {e}", fl!("cli-error-runtime"));
            return ExitCode::FAILURE;
        }
    };

    match command {
        Commands::Sync { reload, json } => rt.block_on(cmd_sync(reload, json)),
        Commands::Colors { action, json } => rt.block_on(cmd_colors(action, json)),
        Commands::Theme { action } => rt.block_on(cmd_theme(action)),
        Commands::Hooks { action } => rt.block_on(cmd_hooks(action)),
        Commands::Wallpaper { action } => rt.block_on(cmd_wallpaper(action)),
    }
}

async fn cmd_sync(reload: bool, json: bool) -> ExitCode {
    let config = ToolSyncConfig::load();

    let result = match tool_sync::sync_tools(&config).await {
        Ok(r) => r,
        Err(e) => {
            if json {
                eprintln!("{{\"error\": \"{e}\"}}");
            } else {
                eprintln!("{}: {e}", fl!("cli-error-sync-failed"));
            }
            return ExitCode::FAILURE;
        }
    };

    let tools_synced = result.synced_tools.clone();

    let hooks_output = result.hooks_result.as_ref().map(HooksOutput::from);

    let signal_result = if reload {
        tool_sync::signal_running_apps(&config)
    } else {
        tool_sync::SignalResult::default()
    };

    if json {
        let output = SyncOutput {
            colors_path: result.colors_path.display().to_string(),
            tools_synced,
            hooks: hooks_output,
            apps_reloaded: signal_result.reloaded,
            apps_manual: signal_result.skipped,
        };
        print_json(&output);
    } else {
        println!("{}", fl!("cli-sync-complete"));
        println!("  colors: {}", result.colors_path.display());
        if !tools_synced.is_empty() {
            println!("  tools: {}", tools_synced.join(", "));
        }
        if let Some(ref h) = result.hooks_result
            && h.hooks_run > 0
        {
            println!("  hooks: {}/{} succeeded", h.hooks_succeeded, h.hooks_run);
        }
        if !signal_result.reloaded.is_empty() {
            println!("  reloaded: {}", signal_result.reloaded.join(", "));
        }
        if !signal_result.skipped.is_empty() {
            println!("  manual: {}", signal_result.skipped.join(", "));
        }
    }

    ExitCode::SUCCESS
}

async fn cmd_colors(action: Option<ColorsAction>, json: bool) -> ExitCode {
    let palette = ColorPalette::from_cosmic();

    match action {
        Some(ColorsAction::Save { path }) => {
            let result = if let Some(ref p) = path {
                let config_dir = p.parent().unwrap_or(p);
                if let Err(e) = tokio::fs::create_dir_all(config_dir).await {
                    eprintln!("{}: {e}", fl!("cli-error-save-failed"));
                    return ExitCode::FAILURE;
                }
                tokio::fs::write(p, palette.to_toml())
                    .await
                    .map(|()| p.clone())
            } else {
                palette.save().await
            };

            match result {
                Ok(saved_path) => {
                    if json {
                        println!("{{\"path\": \"{}\"}}", saved_path.display());
                    } else {
                        println!("{}: {}", fl!("cli-colors-saved"), saved_path.display());
                    }
                    ExitCode::SUCCESS
                }
                Err(e) => {
                    eprintln!("{}: {e}", fl!("cli-error-save-failed"));
                    ExitCode::FAILURE
                }
            }
        }
        None => {
            if json {
                let output = ColorsOutput::from(&palette);
                print_json(&output);
            } else {
                print!("{}", palette.to_toml());
            }
            ExitCode::SUCCESS
        }
    }
}

async fn cmd_theme(action: ThemeAction) -> ExitCode {
    match action {
        ThemeAction::Info { json } => {
            let config = ThemeConfig::load();
            let accent_hex = colors::srgba_to_hex(&config.accent_color);
            let mode = if config.is_dark { "dark" } else { "light" };

            if json {
                let output = ThemeInfoOutput {
                    name: config.name.clone(),
                    mode: mode.to_string(),
                    accent_color: accent_hex.clone(),
                };
                print_json(&output);
            } else {
                println!("{}: {}", fl!("cli-theme-info"), config.name);
                println!("  mode: {mode}");
                println!("  accent: {accent_hex}");
            }
            ExitCode::SUCCESS
        }
        ThemeAction::Dark => match ThemeConfig::set_dark_mode(true) {
            Ok(()) => {
                println!("{}", fl!("cli-theme-dark-set"));
                ExitCode::SUCCESS
            }
            Err(e) => {
                eprintln!("{e}");
                ExitCode::FAILURE
            }
        },
        ThemeAction::Light => match ThemeConfig::set_dark_mode(false) {
            Ok(()) => {
                println!("{}", fl!("cli-theme-light-set"));
                ExitCode::SUCCESS
            }
            Err(e) => {
                eprintln!("{e}");
                ExitCode::FAILURE
            }
        },
        ThemeAction::SetAccent { hex } => {
            let Some((r, g, b)) = colors::hex_to_rgb(&hex) else {
                eprintln!("{}: {hex}", fl!("cli-error-invalid-hex"));
                return ExitCode::FAILURE;
            };

            let config = ThemeConfig::load();
            match ThemeConfig::set_accent_color(
                f32::from(r) / 255.0,
                f32::from(g) / 255.0,
                f32::from(b) / 255.0,
                config.is_dark,
            ) {
                Ok(()) => {
                    println!("{}: {hex}", fl!("cli-theme-accent-set"));
                    ExitCode::SUCCESS
                }
                Err(e) => {
                    eprintln!("{e}");
                    ExitCode::FAILURE
                }
            }
        }
        ThemeAction::Export { path } => match ThemeConfig::export_theme(&path).await {
            Ok(exported) => {
                println!("{}: {exported}", fl!("cli-theme-exported"));
                ExitCode::SUCCESS
            }
            Err(e) => {
                eprintln!("{}: {e}", fl!("cli-error-export-failed"));
                ExitCode::FAILURE
            }
        },
        ThemeAction::Import { path } => match ThemeConfig::import_theme(&path).await {
            Ok(imported) => {
                println!("{}: {imported}", fl!("cli-theme-imported"));
                ExitCode::SUCCESS
            }
            Err(e) => {
                eprintln!("{}: {e}", fl!("cli-error-import-failed"));
                ExitCode::FAILURE
            }
        },
    }
}

async fn cmd_hooks(action: HooksAction) -> ExitCode {
    match action {
        HooksAction::Run { json } => {
            let palette = ColorPalette::from_cosmic();
            let colors_path = match palette.save().await {
                Ok(p) => p,
                Err(e) => {
                    eprintln!("{}: {e}", fl!("cli-error-save-failed"));
                    return ExitCode::FAILURE;
                }
            };

            let results = hooks::run_hooks(&palette, &colors_path).await;

            if json {
                let output = HooksOutput::from(&results);
                print_json(&output);
            } else {
                println!("{}", fl!("cli-hooks-complete"));
                println!(
                    "  run: {}, succeeded: {}, failed: {}, timed_out: {}",
                    results.hooks_run,
                    results.hooks_succeeded,
                    results.hooks_failed,
                    results.hooks_timed_out
                );
            }

            if results.hooks_failed > 0 || results.hooks_timed_out > 0 {
                ExitCode::FAILURE
            } else {
                ExitCode::SUCCESS
            }
        }
    }
}

async fn cmd_wallpaper(action: WallpaperAction) -> ExitCode {
    match action {
        WallpaperAction::Add { url, json, set } => {
            let parsed = match reqwest::Url::parse(&url) {
                Ok(u) => u,
                Err(e) => {
                    eprintln!("{}: {e}", fl!("cli-error-download-failed"));
                    return ExitCode::FAILURE;
                }
            };

            let response = match reqwest::get(parsed).await {
                Ok(r) => r,
                Err(e) => {
                    eprintln!("{}: {e}", fl!("cli-error-download-failed"));
                    return ExitCode::FAILURE;
                }
            };

            if !response.status().is_success() {
                eprintln!(
                    "{}: HTTP {}",
                    fl!("cli-error-download-failed"),
                    response.status()
                );
                return ExitCode::FAILURE;
            }

            // Extract filename from URL path
            let url_path = response.url().path();
            let filename = url_path
                .rsplit('/')
                .next()
                .filter(|s| !s.is_empty() && s.contains('.'))
                .unwrap_or("downloaded-wallpaper.png");

            // Sanitize filename: keep only alphanumeric, dots, hyphens, underscores
            let safe_filename: String = filename
                .chars()
                .map(|c| {
                    if c.is_alphanumeric() || c == '.' || c == '-' || c == '_' {
                        c
                    } else {
                        '_'
                    }
                })
                .collect();

            // Destination: ~/.local/share/backgrounds/custom/
            let dest_dir = directories::BaseDirs::new().map_or_else(
                || PathBuf::from("backgrounds/custom"),
                |b| b.data_local_dir().join("backgrounds/custom"),
            );

            if let Err(e) = tokio::fs::create_dir_all(&dest_dir).await {
                eprintln!("{}: {e}", fl!("cli-error-download-failed"));
                return ExitCode::FAILURE;
            }

            let dest_path = dest_dir.join(&safe_filename);

            let bytes = match response.bytes().await {
                Ok(b) => b,
                Err(e) => {
                    eprintln!("{}: {e}", fl!("cli-error-download-failed"));
                    return ExitCode::FAILURE;
                }
            };

            // Write to a temp file first, then validate
            let tmp_path = dest_path.with_extension("tmp");
            if let Err(e) = tokio::fs::write(&tmp_path, &bytes).await {
                eprintln!("{}: {e}", fl!("cli-error-download-failed"));
                return ExitCode::FAILURE;
            }

            // Validate that it's a real image (guess format from content, not extension)
            let tmp_clone = tmp_path.clone();
            let valid = tokio::task::spawn_blocking(move || {
                image::ImageReader::open(&tmp_clone)
                    .ok()
                    .and_then(|r| r.with_guessed_format().ok())
                    .and_then(|r| r.decode().ok())
                    .is_some()
            })
            .await
            .unwrap_or(false);

            if !valid {
                let _ = tokio::fs::remove_file(&tmp_path).await;
                eprintln!("{}", fl!("cli-error-invalid-image"));
                return ExitCode::FAILURE;
            }

            // Move temp file to final location
            if let Err(e) = tokio::fs::rename(&tmp_path, &dest_path).await {
                eprintln!("{}: {e}", fl!("cli-error-download-failed"));
                return ExitCode::FAILURE;
            }

            let saved = dest_path.display().to_string();

            let active_set = if set {
                match set_active_wallpaper(&dest_path) {
                    Ok(()) => true,
                    Err(e) => {
                        eprintln!("warning: failed to set active wallpaper: {e}");
                        false
                    }
                }
            } else {
                false
            };

            if json {
                let output = WallpaperOutput {
                    path: saved,
                    active_set,
                };
                print_json(&output);
            } else {
                println!("{}: {saved}", fl!("cli-wallpaper-added"));
                if active_set {
                    println!("  active wallpaper: set");
                }
            }

            ExitCode::SUCCESS
        }
    }
}

#[derive(Serialize)]
struct WallpaperOutput {
    path: String,
    /// True if --set was passed and the wallpaper was successfully applied
    /// to cosmic-bg's config. False otherwise.
    active_set: bool,
}

/// Apply a downloaded image as the active COSMIC wallpaper by writing
/// to cosmic-bg's cosmic-config files directly.
///
/// Schema (from pop-os/cosmic-bg, v1):
/// - `same-on-all` file contains the RON `true`
/// - `all` file contains a RON-serialized `Background` struct
///
/// Writing the files by hand keeps us off cosmic-bg-config as a build
/// dep. This is best-effort: if the schema changes, the daemon may
/// reject the new value. The caller should treat any error here as
/// non-fatal — the wallpaper file is already saved.
fn set_active_wallpaper(path: &std::path::Path) -> Result<(), String> {
    let abs_path = path
        .canonicalize()
        .map_err(|e| format!("canonicalize: {e}"))?;
    let path_str = abs_path
        .to_str()
        .ok_or_else(|| "wallpaper path is not valid UTF-8".to_string())?;
    if path_str.contains('"') || path_str.contains('\\') {
        return Err("wallpaper path contains characters that need RON escaping".to_string());
    }

    let config_dir = directories::BaseDirs::new()
        .ok_or_else(|| "cannot resolve user config dir".to_string())?
        .config_dir()
        .join("cosmic/com.system76.CosmicBackground/v1");

    std::fs::create_dir_all(&config_dir).map_err(|e| format!("mkdir: {e}"))?;

    let bg_ron = format!(
        "(output: All, source: Path(\"{path_str}\"), filter_by_theme: false, \
         rotation_frequency: 0, filter_method: Lanczos, scaling_mode: Zoom, \
         sampling_method: Aspect)"
    );

    std::fs::write(config_dir.join("same-on-all"), "true")
        .map_err(|e| format!("write same-on-all: {e}"))?;
    std::fs::write(config_dir.join("all"), bg_ron).map_err(|e| format!("write all: {e}"))?;

    Ok(())
}

fn print_json<T: Serialize>(value: &T) {
    // serde_json::to_string_pretty won't fail on simple structs
    if let Ok(s) = serde_json::to_string_pretty(value) {
        println!("{s}");
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    use clap::Parser;

    #[test]
    fn test_cli_no_args_is_gui_mode() {
        let cli = Cli::try_parse_from(["cosmic-order"]).unwrap();
        assert!(cli.command.is_none());
    }

    #[test]
    fn test_cli_sync_subcommand() {
        let cli = Cli::try_parse_from(["cosmic-order", "sync"]).unwrap();
        assert!(matches!(
            cli.command,
            Some(Commands::Sync {
                reload: false,
                json: false
            })
        ));
    }

    #[test]
    fn test_cli_sync_with_flags() {
        let cli = Cli::try_parse_from(["cosmic-order", "sync", "--reload", "--json"]).unwrap();
        assert!(matches!(
            cli.command,
            Some(Commands::Sync {
                reload: true,
                json: true
            })
        ));
    }

    #[test]
    fn test_cli_colors_no_action() {
        let cli = Cli::try_parse_from(["cosmic-order", "colors"]).unwrap();
        assert!(matches!(
            cli.command,
            Some(Commands::Colors {
                action: None,
                json: false
            })
        ));
    }

    #[test]
    fn test_cli_colors_json() {
        let cli = Cli::try_parse_from(["cosmic-order", "colors", "--json"]).unwrap();
        assert!(matches!(
            cli.command,
            Some(Commands::Colors {
                action: None,
                json: true
            })
        ));
    }

    #[test]
    fn test_cli_colors_save() {
        let cli = Cli::try_parse_from(["cosmic-order", "colors", "save"]).unwrap();
        match cli.command {
            Some(Commands::Colors {
                action: Some(ColorsAction::Save { path }),
                ..
            }) => assert!(path.is_none()),
            other => panic!("Expected Colors Save, got {other:?}"),
        }
    }

    #[test]
    fn test_cli_colors_save_with_path() {
        let cli = Cli::try_parse_from(["cosmic-order", "colors", "save", "/tmp/out.toml"]).unwrap();
        match cli.command {
            Some(Commands::Colors {
                action: Some(ColorsAction::Save { path }),
                ..
            }) => assert_eq!(path.unwrap().to_str().unwrap(), "/tmp/out.toml"),
            other => panic!("Expected Colors Save with path, got {other:?}"),
        }
    }

    #[test]
    fn test_cli_theme_info() {
        let cli = Cli::try_parse_from(["cosmic-order", "theme", "info"]).unwrap();
        assert!(matches!(
            cli.command,
            Some(Commands::Theme {
                action: ThemeAction::Info { json: false }
            })
        ));
    }

    #[test]
    fn test_cli_theme_dark() {
        let cli = Cli::try_parse_from(["cosmic-order", "theme", "dark"]).unwrap();
        assert!(matches!(
            cli.command,
            Some(Commands::Theme {
                action: ThemeAction::Dark
            })
        ));
    }

    #[test]
    fn test_cli_theme_light() {
        let cli = Cli::try_parse_from(["cosmic-order", "theme", "light"]).unwrap();
        assert!(matches!(
            cli.command,
            Some(Commands::Theme {
                action: ThemeAction::Light
            })
        ));
    }

    #[test]
    fn test_cli_theme_set_accent() {
        let cli = Cli::try_parse_from(["cosmic-order", "theme", "set-accent", "#FF5733"]).unwrap();
        match cli.command {
            Some(Commands::Theme {
                action: ThemeAction::SetAccent { hex },
            }) => assert_eq!(hex, "#FF5733"),
            other => panic!("Expected SetAccent, got {other:?}"),
        }
    }

    #[test]
    fn test_cli_theme_export() {
        let cli =
            Cli::try_parse_from(["cosmic-order", "theme", "export", "/tmp/theme.ron"]).unwrap();
        match cli.command {
            Some(Commands::Theme {
                action: ThemeAction::Export { path },
            }) => assert_eq!(path.to_str().unwrap(), "/tmp/theme.ron"),
            other => panic!("Expected Export, got {other:?}"),
        }
    }

    #[test]
    fn test_cli_theme_import() {
        let cli =
            Cli::try_parse_from(["cosmic-order", "theme", "import", "/tmp/theme.ron"]).unwrap();
        match cli.command {
            Some(Commands::Theme {
                action: ThemeAction::Import { path },
            }) => assert_eq!(path.to_str().unwrap(), "/tmp/theme.ron"),
            other => panic!("Expected Import, got {other:?}"),
        }
    }

    #[test]
    fn test_cli_hooks_run() {
        let cli = Cli::try_parse_from(["cosmic-order", "hooks", "run"]).unwrap();
        assert!(matches!(
            cli.command,
            Some(Commands::Hooks {
                action: HooksAction::Run { json: false }
            })
        ));
    }

    #[test]
    fn test_cli_hooks_run_json() {
        let cli = Cli::try_parse_from(["cosmic-order", "hooks", "run", "--json"]).unwrap();
        assert!(matches!(
            cli.command,
            Some(Commands::Hooks {
                action: HooksAction::Run { json: true }
            })
        ));
    }

    #[test]
    fn test_cli_wallpaper_add() {
        let cli = Cli::try_parse_from([
            "cosmic-order",
            "wallpaper",
            "add",
            "https://example.com/wall.png",
        ])
        .unwrap();
        match cli.command {
            Some(Commands::Wallpaper {
                action: WallpaperAction::Add { url, json, set },
            }) => {
                assert_eq!(url, "https://example.com/wall.png");
                assert!(!json);
                assert!(!set);
            }
            other => panic!("Expected Wallpaper Add, got {other:?}"),
        }
    }

    #[test]
    fn test_cli_wallpaper_add_set() {
        let cli = Cli::try_parse_from([
            "cosmic-order",
            "wallpaper",
            "add",
            "https://example.com/wall.png",
            "--set",
        ])
        .unwrap();
        match cli.command {
            Some(Commands::Wallpaper {
                action: WallpaperAction::Add { set, .. },
            }) => assert!(set),
            other => panic!("Expected Wallpaper Add, got {other:?}"),
        }
    }

    #[test]
    fn test_cli_invalid_subcommand() {
        let result = Cli::try_parse_from(["cosmic-order", "nonexistent"]);
        assert!(result.is_err());
    }

    #[test]
    fn test_cli_theme_requires_action() {
        let result = Cli::try_parse_from(["cosmic-order", "theme"]);
        assert!(result.is_err());
    }

    #[test]
    fn test_colors_output_from_palette() {
        let palette = crate::colors::ColorPalette {
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
        };
        let output = ColorsOutput::from(&palette);
        assert_eq!(output.accent, "#63D1D6");
        assert_eq!(output.background, "#1B1B1B");
        assert_eq!(output.colors.len(), 16);
    }

    #[test]
    fn test_hooks_output_from_results() {
        let results = crate::hooks::HookResults {
            hooks_run: 5,
            hooks_succeeded: 3,
            hooks_failed: 1,
            hooks_timed_out: 1,
        };
        let output = HooksOutput::from(&results);
        assert_eq!(output.hooks_run, 5);
        assert_eq!(output.hooks_succeeded, 3);
        assert_eq!(output.hooks_failed, 1);
        assert_eq!(output.hooks_timed_out, 1);
    }

    #[test]
    fn test_sync_output_serializes() {
        let output = SyncOutput {
            colors_path: "/tmp/colors.toml".to_string(),
            tools_synced: vec!["Ghostty".to_string(), "btop".to_string()],
            hooks: Some(HooksOutput {
                hooks_run: 2,
                hooks_succeeded: 2,
                hooks_failed: 0,
                hooks_timed_out: 0,
            }),
            apps_reloaded: vec!["ghostty".to_string()],
            apps_manual: vec!["fzf (re-source shell)".to_string()],
        };
        let json = serde_json::to_string_pretty(&output).unwrap();
        assert!(json.contains("colors_path"));
        assert!(json.contains("apps_manual"));
        assert!(json.contains("Ghostty"));
        assert!(json.contains("hooks_run"));
        assert!(json.contains("apps_reloaded"));
    }

    #[test]
    fn test_theme_info_output_serializes() {
        let output = ThemeInfoOutput {
            name: "COSMIC Dark".to_string(),
            mode: "dark".to_string(),
            accent_color: "#63D1D6".to_string(),
        };
        let json = serde_json::to_string(&output).unwrap();
        assert!(json.contains("COSMIC Dark"));
        assert!(json.contains("dark"));
        assert!(json.contains("#63D1D6"));
    }

    #[test]
    fn test_wallpaper_output_serializes() {
        let output = WallpaperOutput {
            path: "/home/user/backgrounds/wall.png".to_string(),
            active_set: false,
        };
        let json = serde_json::to_string(&output).unwrap();
        assert!(json.contains("wall.png"));
        assert!(json.contains("active_set"));
    }
}
