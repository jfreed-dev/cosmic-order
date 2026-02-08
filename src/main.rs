// SPDX-License-Identifier: GPL-3.0-only

//! COSMIC ORDER - Establishing order in the chaos
//!
//! OMARCHY-inspired workflow and aesthetics for COSMIC Desktop.
//! The keyboard-first workflow you love, on the desktop you deserve.

mod app;
mod compositor;
mod config;
mod cosmic_idle;
mod inhibit;
mod localize;
mod pages;
mod power;
mod screensaver_config;
mod systemd;
mod theme_config;
mod wallpaper_config;

use tracing_subscriber::{EnvFilter, layer::SubscriberExt, util::SubscriberInitExt};

/// Application ID for COSMIC configuration
pub const APP_ID: &str = "com.github.jfreed-dev.CosmicOrder";

fn main() -> cosmic::iced::Result {
    // Initialize logging
    tracing_subscriber::registry()
        .with(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new("warn,cosmic_order=info")),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    tracing::info!("Starting COSMIC ORDER");

    // Initialize localization with system language preferences
    let requested_languages = i18n_embed::DesktopLanguageRequester::requested_languages();
    localize::init(&requested_languages);

    // Run the application
    cosmic::app::run::<app::App>(cosmic::app::Settings::default(), ())
}
