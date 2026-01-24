// SPDX-License-Identifier: GPL-3.0-only

//! Localization support using Fluent
//!
//! Provides the `fl!` macro for translating strings.

use i18n_embed::{
    fluent::{fluent_language_loader, FluentLanguageLoader},
    DesktopLanguageRequester,
};
use once_cell::sync::Lazy;
use rust_embed::RustEmbed;

#[derive(RustEmbed)]
#[folder = "resources/i18n/"]
struct Localizations;

/// Language loader for translations
pub static LANGUAGE_LOADER: Lazy<FluentLanguageLoader> = Lazy::new(|| {
    let loader = fluent_language_loader!();
    let requested_languages = DesktopLanguageRequester::requested_languages();
    let _result = i18n_embed::select(&loader, &Localizations, &requested_languages);
    loader
});

/// Initialize localization
pub fn init() {
    // Force initialization of the lazy loader
    let _ = &*LANGUAGE_LOADER;
}

/// Translation macro
///
/// # Examples
///
/// ```
/// let title = fl!("app-title");
/// let greeting = fl!("greeting", name = "User");
/// ```
#[macro_export]
macro_rules! fl {
    ($message_id:literal) => {{
        i18n_embed_fl::fl!($crate::localize::LANGUAGE_LOADER, $message_id)
    }};
    ($message_id:literal, $($arg:tt)*) => {{
        i18n_embed_fl::fl!($crate::localize::LANGUAGE_LOADER, $message_id, $($arg)*)
    }};
}

pub use fl;
