// SPDX-License-Identifier: GPL-3.0-only

//! COSMIC Tweaks - Theme, wallpaper, and screensaver management
//!
//! A native COSMIC Desktop application for managing themes, wallpapers,
//! and screensaver configurations.

mod app;
mod config;
mod localize;
mod pages;

use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

/// Application ID for COSMIC configuration
pub const APP_ID: &str = "com.github.jfreed-dev.CosmicTweaks";

fn main() -> cosmic::iced::Result {
    // Initialize logging
    tracing_subscriber::registry()
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| {
            EnvFilter::new("warn,cosmic_tweaks=info")
        }))
        .with(tracing_subscriber::fmt::layer())
        .init();

    tracing::info!("Starting COSMIC Tweaks");

    // Initialize localization with system language preferences
    let requested_languages = i18n_embed::DesktopLanguageRequester::requested_languages();
    localize::init(&requested_languages);

    // Run the application
    cosmic::app::run::<app::App>(cosmic::app::Settings::default(), ())
}
