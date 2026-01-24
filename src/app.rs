// SPDX-License-Identifier: GPL-3.0-only

//! Main application module
//!
//! Implements the COSMIC Application trait and handles message routing.

use cosmic::app::{Core, Task};
use cosmic::widget::{self, nav_bar};
use cosmic::{Application, Element};

use crate::config::Config;
use crate::fl;
use crate::pages::{self, PageId};
use crate::screensaver_config::ScreensaverConfig;
use crate::theme_config::ThemeConfig;

/// Application state
pub struct App {
    /// Core COSMIC application state
    core: Core,
    /// Application configuration
    config: Config,
    /// Navigation bar model
    nav_model: nav_bar::Model,
    /// Currently active page
    active_page: PageId,
    /// Screensaver configuration
    screensaver_config: ScreensaverConfig,
    /// Theme configuration
    theme_config: ThemeConfig,
}

/// Application messages
#[derive(Debug, Clone)]
#[allow(dead_code)] // Variants will be used as features are implemented
pub enum Message {
    /// Navigation item selected
    NavSelect(nav_bar::Id),
    /// Page-specific message
    Page(pages::Message),
    /// Configuration changed
    ConfigChanged(Config),
}

impl Application for App {
    type Message = Message;
    type Executor = cosmic::executor::Default;
    type Flags = ();

    const APP_ID: &'static str = crate::APP_ID;

    fn core(&self) -> &Core {
        &self.core
    }

    fn core_mut(&mut self) -> &mut Core {
        &mut self.core
    }

    fn init(core: Core, _flags: Self::Flags) -> (Self, Task<Message>) {
        // Load configuration
        let config = Config::load().unwrap_or_default();

        // Load screensaver configuration
        let screensaver_config = ScreensaverConfig::load().unwrap_or_default();
        tracing::info!("Loaded screensaver config: enabled={}", screensaver_config.enabled);

        // Load theme configuration
        let theme_config = ThemeConfig::load();
        tracing::info!(
            "Loaded theme config: name={}, dark={}",
            theme_config.name,
            theme_config.is_dark
        );

        // Build navigation model
        let mut nav_model = nav_bar::Model::default();

        // Add navigation items
        nav_model
            .insert()
            .text(fl!("themes"))
            .icon(widget::icon::from_name(
                "preferences-desktop-theme-symbolic",
            ))
            .data(PageId::Themes);

        nav_model
            .insert()
            .text(fl!("wallpapers"))
            .icon(widget::icon::from_name(
                "preferences-desktop-wallpaper-symbolic",
            ))
            .data(PageId::Wallpapers);

        nav_model
            .insert()
            .text(fl!("screensaver"))
            .icon(widget::icon::from_name(
                "preferences-desktop-screensaver-symbolic",
            ))
            .data(PageId::Screensaver);

        // Activate first item
        nav_model.activate_position(0);

        let app = Self {
            core,
            config,
            nav_model,
            active_page: PageId::Themes,
            screensaver_config,
            theme_config,
        };

        (app, Task::none())
    }

    fn nav_model(&self) -> Option<&nav_bar::Model> {
        Some(&self.nav_model)
    }

    fn on_nav_select(&mut self, id: nav_bar::Id) -> Task<Message> {
        // Get the page ID from the navigation item
        if let Some(page_id) = self.nav_model.data::<PageId>(id).cloned() {
            self.active_page = page_id;
            self.nav_model.activate(id);
        }
        Task::none()
    }

    fn update(&mut self, message: Self::Message) -> Task<Message> {
        match message {
            Message::NavSelect(id) => self.on_nav_select(id),
            Message::Page(page_message) => {
                // Route to appropriate page
                // TODO: Implement page message routing
                tracing::debug!("Page message: {:?}", page_message);
                Task::none()
            }
            Message::ConfigChanged(config) => {
                self.config = config;
                Task::none()
            }
        }
    }

    fn view(&self) -> Element<'_, Self::Message> {
        // Build page content based on active page
        match self.active_page {
            PageId::Themes => self.view_themes_page(),
            PageId::Wallpapers => self.view_wallpapers_page(),
            PageId::Screensaver => self.view_screensaver_page(),
        }
    }

    fn header_start(&self) -> Vec<Element<'_, Self::Message>> {
        vec![]
    }

    fn header_center(&self) -> Vec<Element<'_, Self::Message>> {
        vec![widget::text::title3(fl!("app-title")).into()]
    }

    fn header_end(&self) -> Vec<Element<'_, Self::Message>> {
        vec![]
    }
}

impl App {
    /// View for the Themes page
    fn view_themes_page(&self) -> Element<'_, Message> {
        let spacing = cosmic::theme::spacing();
        let cfg = &self.theme_config;

        widget::column()
            .spacing(spacing.space_m)
            .padding(spacing.space_m)
            .push(widget::text::title2(fl!("themes")))
            .push(widget::text::body(fl!("themes-description")))
            // Current theme section
            .push(
                widget::settings::section()
                    .title("Current Theme")
                    .add(widget::settings::item(
                        "Name",
                        widget::text::body(&cfg.name),
                    ))
                    .add(widget::settings::item(
                        "Mode",
                        widget::text::body(if cfg.is_dark { "Dark" } else { "Light" }),
                    )),
            )
            // Colors section
            .push(
                widget::settings::section()
                    .title("Colors")
                    .add(widget::settings::item(
                        "Accent",
                        widget::text::body(cfg.accent_hex()),
                    ))
                    .add(widget::settings::item(
                        "Background",
                        widget::text::body(cfg.background_hex()),
                    ))
                    .add(widget::settings::item(
                        "Text",
                        widget::text::body(cfg.text_hex()),
                    )),
            )
            // Coming soon section
            .push(
                widget::settings::section()
                    .title("Coming Soon")
                    .add(widget::settings::item(
                        "Theme switching",
                        widget::text::body("Coming in Phase 2"),
                    ))
                    .add(widget::settings::item(
                        "Color customization",
                        widget::text::body("Coming in Phase 2"),
                    )),
            )
            .into()
    }

    /// View for the Wallpapers page
    #[allow(clippy::unused_self)] // Will use self when wallpaper data is added
    fn view_wallpapers_page(&self) -> Element<'_, Message> {
        let spacing = cosmic::theme::spacing();

        widget::column()
            .spacing(spacing.space_m)
            .padding(spacing.space_m)
            .push(widget::text::title2(fl!("wallpapers")))
            .push(widget::text::body(fl!("wallpapers-description")))
            .push(
                widget::settings::section()
                    .title("Coming Soon")
                    .add(widget::settings::item(
                        "Wallpaper management",
                        widget::text::body("Coming in Phase 3"),
                    )),
            )
            .into()
    }

    /// View for the Screensaver page
    fn view_screensaver_page(&self) -> Element<'_, Message> {
        let spacing = cosmic::theme::spacing();
        let cfg = &self.screensaver_config;

        widget::column()
            .spacing(spacing.space_m)
            .padding(spacing.space_m)
            .push(widget::text::title2(fl!("screensaver")))
            .push(widget::text::body(fl!("screensaver-description")))
            // Status section
            .push(
                widget::settings::section()
                    .title("Status")
                    .add(widget::settings::item(
                        "Screensaver",
                        widget::text::body(if cfg.enabled { "Enabled" } else { "Disabled" }),
                    ))
                    .add(widget::settings::item(
                        "Terminal",
                        widget::text::body(&cfg.terminal),
                    ))
                    .add(widget::settings::item(
                        "Logo",
                        widget::text::body(cfg.logo_name()),
                    )),
            )
            // Timeouts section
            .push(
                widget::settings::section()
                    .title("Timeouts")
                    .add(widget::settings::item(
                        "Idle timeout",
                        widget::text::body(ScreensaverConfig::format_timeout(cfg.idle_timeout)),
                    ))
                    .add(widget::settings::item(
                        "Lock timeout",
                        widget::text::body(ScreensaverConfig::format_timeout(cfg.lock_timeout)),
                    ))
                    .add(widget::settings::item(
                        "Screen off (DPMS)",
                        widget::text::body(ScreensaverConfig::format_timeout(cfg.dpms_timeout)),
                    )),
            )
            // Effects section
            .push(
                widget::settings::section()
                    .title("Effects")
                    .add(widget::settings::item(
                        "Frame rate",
                        widget::text::body(format!("{} fps", cfg.frame_rate)),
                    ))
                    .add(widget::settings::item(
                        "Excluded effects",
                        widget::text::body(if cfg.exclude_effects.is_empty() {
                            "None".to_string()
                        } else {
                            cfg.exclude_effects.clone()
                        }),
                    ))
                    .add(widget::settings::item(
                        "Included effects",
                        widget::text::body(if cfg.include_effects.is_empty() {
                            "All (except excluded)".to_string()
                        } else {
                            cfg.include_effects.clone()
                        }),
                    )),
            )
            // Clock section
            .push(
                widget::settings::section()
                    .title("Clock Display")
                    .add(widget::settings::item(
                        "Show clock",
                        widget::text::body(if cfg.show_clock { "Yes" } else { "No" }),
                    ))
                    .add(widget::settings::item(
                        "Clock duration",
                        widget::text::body(format!("{} seconds", cfg.clock_duration)),
                    ))
                    .add(widget::settings::item(
                        "Clock format",
                        widget::text::body(&cfg.clock_format),
                    )),
            )
            // Power section
            .push(
                widget::settings::section()
                    .title("Power Settings")
                    .add(widget::settings::item(
                        "Disable on battery",
                        widget::text::body(if cfg.disable_on_battery { "Yes" } else { "No" }),
                    ))
                    .add(widget::settings::item(
                        "Battery idle timeout",
                        widget::text::body(ScreensaverConfig::format_timeout(cfg.battery_idle_timeout)),
                    )),
            )
            .into()
    }
}
