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

/// Cosmic Enhancements — OMARCHY-inspired workflow and aesthetics for COSMIC Desktop
#[derive(Parser)]
#[command(name = "cosmic-order", version, about)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand)]
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
}

#[derive(Subcommand)]
pub enum ColorsAction {
    /// Save colors.toml to disk (default: ~/.config/cosmic-order/colors.toml)
    Save {
        /// Custom output path
        path: Option<PathBuf>,
    },
}

#[derive(Subcommand)]
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

#[derive(Subcommand)]
pub enum HooksAction {
    /// Run all hooks with the current color palette
    Run {
        /// Output results as JSON
        #[arg(long)]
        json: bool,
    },
}

// --- JSON output structs ---

#[derive(Serialize)]
struct SyncOutput {
    colors_path: String,
    tools_synced: Vec<String>,
    hooks: Option<HooksOutput>,
    apps_reloaded: Vec<String>,
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

    let mut tools_synced = Vec::new();
    if result.ghostty_synced {
        tools_synced.push("Ghostty".to_string());
    }
    if result.btop_synced {
        tools_synced.push("btop".to_string());
    }
    if result.nvim_synced {
        tools_synced.push("Neovim".to_string());
    }
    if result.zellij_synced {
        tools_synced.push("Zellij".to_string());
    }
    if result.fzf_synced {
        tools_synced.push("fzf".to_string());
    }
    if result.lazygit_synced {
        tools_synced.push("lazygit".to_string());
    }

    let hooks_output = result.hooks_result.as_ref().map(HooksOutput::from);

    let apps_reloaded = if reload {
        tool_sync::signal_running_apps(&config)
    } else {
        Vec::new()
    };

    if json {
        let output = SyncOutput {
            colors_path: result.colors_path.display().to_string(),
            tools_synced,
            hooks: hooks_output,
            apps_reloaded,
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
        if !apps_reloaded.is_empty() {
            println!("  reloaded: {}", apps_reloaded.join(", "));
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
            let accent_hex = ThemeConfig::color_to_hex(&config.accent_color);
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

fn print_json<T: Serialize>(value: &T) {
    // serde_json::to_string_pretty won't fail on simple structs
    if let Ok(s) = serde_json::to_string_pretty(value) {
        println!("{s}");
    }
}
