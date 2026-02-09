// SPDX-License-Identifier: GPL-3.0-only

//! COSMIC ORDER - Establishing order in the chaos
//!
//! OMARCHY-inspired workflow and aesthetics for COSMIC Desktop.
//! The keyboard-first workflow you love, on the desktop you deserve.

mod app;
mod bundled_themes;
mod cli;
mod colors;
mod compositor;
mod config;
mod cosmic_idle;
mod generators;
mod hooks;
mod inhibit;
mod localize;
mod pages;
mod power;
mod screensaver_config;
mod sleep_lock;
mod systemd;
mod theme_config;
mod tool_sync;
mod wallpaper_config;
mod wayland_idle;

use std::process::ExitCode;

use clap::Parser;
use tracing_subscriber::{EnvFilter, layer::SubscriberExt, util::SubscriberInitExt};

/// Application ID for COSMIC configuration
pub const APP_ID: &str = "com.github.jfreed-dev.CosmicOrder";

fn main() -> ExitCode {
    // Initialize logging
    tracing_subscriber::registry()
        .with(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new("warn,cosmic_order=info")),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    // Initialize localization with system language preferences
    let requested_languages = i18n_embed::DesktopLanguageRequester::requested_languages();
    localize::init(&requested_languages);

    // Parse CLI arguments
    let args = cli::Cli::parse();

    if let Some(cmd) = args.command {
        cli::run(cmd)
    } else {
        tracing::info!("Starting Cosmic Enhancements");
        match cosmic::app::run::<app::App>(cosmic::app::Settings::default(), ()) {
            Ok(()) => ExitCode::SUCCESS,
            Err(e) => {
                tracing::error!("Application error: {e}");
                ExitCode::FAILURE
            }
        }
    }
}
