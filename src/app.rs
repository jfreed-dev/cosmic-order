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
use crate::wallpaper_config::WallpaperConfig;

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
    /// Wallpaper configuration
    wallpaper_config: WallpaperConfig,
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
        tracing::info!(
            "Loaded screensaver config: enabled={}",
            screensaver_config.enabled
        );

        // Load theme configuration
        let theme_config = ThemeConfig::load();
        tracing::info!(
            "Loaded theme config: name={}, dark={}",
            theme_config.name,
            theme_config.is_dark
        );

        // Load wallpaper configuration
        let wallpaper_config = WallpaperConfig::load();
        tracing::info!(
            "Loaded wallpaper config: {} themes, {} total wallpapers",
            wallpaper_config.available_themes.len(),
            wallpaper_config.total_wallpaper_count()
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

        // Activate the saved page from config
        let active_page = config.active_page;
        let position = match active_page {
            PageId::Themes => 0,
            PageId::Wallpapers => 1,
            PageId::Screensaver => 2,
        };
        nav_model.activate_position(position);

        let app = Self {
            core,
            config,
            nav_model,
            active_page,
            screensaver_config,
            theme_config,
            wallpaper_config,
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

            // Persist the active page
            self.config.active_page = page_id;
            if let Err(e) = self.config.save() {
                tracing::warn!("Failed to save config: {e}");
            }
        }
        Task::none()
    }

    fn update(&mut self, message: Self::Message) -> Task<Message> {
        match message {
            Message::NavSelect(id) => self.on_nav_select(id),
            Message::Page(page_message) => self.handle_page_message(page_message),
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
    /// Handle page-specific messages
    fn handle_page_message(&mut self, message: pages::Message) -> Task<Message> {
        match message {
            pages::Message::Themes(msg) => self.handle_themes_message(msg),
            pages::Message::Wallpapers(_msg) => {
                // TODO: Implement wallpaper message handling
                Task::none()
            }
            pages::Message::Screensaver(_msg) => {
                // TODO: Implement screensaver message handling
                Task::none()
            }
        }
    }

    /// Handle theme page messages
    fn handle_themes_message(&mut self, message: pages::ThemesMessage) -> Task<Message> {
        match message {
            pages::ThemesMessage::SetDarkMode(is_dark) => {
                if let Err(e) = crate::theme_config::ThemeConfig::set_dark_mode(is_dark) {
                    tracing::error!("Failed to set dark mode: {e}");
                } else {
                    // Optimistically update local state (system theme updates async)
                    self.theme_config.is_dark = is_dark;
                    // Update theme name to match mode
                    self.theme_config.name = if is_dark {
                        "COSMIC Dark".to_string()
                    } else {
                        "COSMIC Light".to_string()
                    };
                    tracing::info!("Dark mode set to: {is_dark}");
                }
                Task::none()
            }
            pages::ThemesMessage::SetAccentColor(r, g, b) => {
                let is_dark = self.theme_config.is_dark;
                if let Err(e) = crate::theme_config::ThemeConfig::set_accent_color(r, g, b, is_dark)
                {
                    tracing::error!("Failed to set accent color: {e}");
                } else {
                    // Optimistically update local state
                    self.theme_config.accent_color =
                        cosmic::cosmic_theme::palette::Srgba::new(r, g, b, 1.0);
                    tracing::info!("Accent color set to: ({r}, {g}, {b})");
                }
                Task::none()
            }
            pages::ThemesMessage::SelectTheme(theme_id) => {
                let previews = crate::theme_config::ThemePreview::built_in_themes();
                if let Some(preview) = previews.iter().find(|p| p.id == theme_id) {
                    if let Err(e) = preview.apply() {
                        tracing::error!("Failed to apply theme: {e}");
                    } else {
                        // Update local state
                        self.theme_config.is_dark = preview.is_dark;
                        self.theme_config.name = preview.name.clone();
                        tracing::info!("Applied theme: {}", preview.name);
                    }
                }
                Task::none()
            }
            pages::ThemesMessage::Export | pages::ThemesMessage::Import => {
                // TODO: Implement theme export/import
                Task::none()
            }
        }
    }

    /// View for the Themes page
    fn view_themes_page(&self) -> Element<'_, Message> {
        let spacing = cosmic::theme::spacing();
        let cfg = &self.theme_config;

        widget::column()
            .spacing(spacing.space_m)
            .padding(spacing.space_m)
            .push(widget::text::title2(fl!("themes")))
            .push(widget::text::body(fl!("themes-description")))
            // Theme presets
            .push(widget::settings::section().title(fl!("theme-presets")).add(
                widget::settings::item(fl!("available-themes"), self.view_theme_list()),
            ))
            // Mode selection
            .push(
                widget::settings::section()
                    .title(fl!("theme-mode"))
                    .add(widget::settings::item(
                        fl!("dark-mode"),
                        widget::toggler(cfg.is_dark).on_toggle(|enabled| {
                            Message::Page(pages::Message::Themes(
                                pages::ThemesMessage::SetDarkMode(enabled),
                            ))
                        }),
                    )),
            )
            // Accent color selection
            .push(
                widget::settings::section()
                    .title(fl!("theme-accent-color"))
                    .add(widget::settings::item(
                        fl!("accent-presets"),
                        self.view_accent_color_presets(),
                    )),
            )
            .into()
    }

    /// Create accent color preset buttons
    fn view_accent_color_presets(&self) -> Element<'_, Message> {
        use cosmic::iced::Length;

        // Current accent color for comparison
        let current = &self.theme_config.accent_color;

        // Preset accent colors (COSMIC-style palette)
        let presets: [(f32, f32, f32, &str); 8] = [
            (0.39, 0.82, 0.87, "Cyan"),   // COSMIC default cyan
            (0.53, 0.59, 0.93, "Blue"),   // Blue
            (0.67, 0.47, 0.82, "Purple"), // Purple
            (0.93, 0.47, 0.62, "Pink"),   // Pink
            (0.93, 0.53, 0.53, "Red"),    // Red
            (0.93, 0.68, 0.47, "Orange"), // Orange
            (0.87, 0.82, 0.47, "Yellow"), // Yellow
            (0.53, 0.82, 0.53, "Green"),  // Green
        ];

        // Use fixed spacing to avoid theme differences
        let mut row = widget::row().spacing(4);

        for (r, g, b, _name) in presets {
            let color = cosmic::iced::Color::from_rgb(r, g, b);
            let msg = Message::Page(pages::Message::Themes(
                pages::ThemesMessage::SetAccentColor(r, g, b),
            ));

            // Check if this color is approximately the current accent
            let is_selected = (current.red - r).abs() < 0.05
                && (current.green - g).abs() < 0.05
                && (current.blue - b).abs() < 0.05;

            row = row.push(
                widget::button::custom(
                    widget::container(widget::Space::new(Length::Fixed(18.0), Length::Fixed(18.0)))
                        .width(Length::Fixed(18.0))
                        .height(Length::Fixed(18.0))
                        .class(cosmic::theme::Container::custom(move |_| {
                            widget::container::Style {
                                background: Some(cosmic::iced::Background::Color(color)),
                                border: cosmic::iced::Border {
                                    radius: 3.0.into(),
                                    width: if is_selected { 2.0 } else { 0.0 },
                                    color: cosmic::iced::Color::WHITE,
                                },
                                ..Default::default()
                            }
                        })),
                )
                .width(Length::Fixed(22.0))
                .height(Length::Fixed(22.0))
                .padding(2)
                .on_press(msg),
            );
        }

        row.into()
    }

    /// Create theme list with color previews
    fn view_theme_list(&self) -> Element<'_, Message> {
        use cosmic::iced::Length;
        let spacing = cosmic::theme::spacing();

        // Only show Dark and Light (skip high contrast for now)
        let themes: Vec<_> = crate::theme_config::ThemePreview::built_in_themes()
            .into_iter()
            .filter(|t| !t.is_high_contrast)
            .collect();

        let mut row = widget::row().spacing(spacing.space_m);

        for preview in themes {
            let accent = cosmic::iced::Color::from_rgb(
                preview.accent.red,
                preview.accent.green,
                preview.accent.blue,
            );
            let background = cosmic::iced::Color::from_rgb(
                preview.background.red,
                preview.background.green,
                preview.background.blue,
            );
            let text_color = cosmic::iced::Color::from_rgb(
                preview.text.red,
                preview.text.green,
                preview.text.blue,
            );

            let theme_id = preview.id;
            let is_current = self.theme_config.is_dark == preview.is_dark;
            let display_name = if preview.is_dark { "Dark" } else { "Light" };

            // Theme card with color swatches
            let card = widget::column()
                .spacing(spacing.space_xxs)
                .width(Length::Fixed(80.0))
                .align_x(cosmic::iced::Alignment::Center)
                .push(
                    // Color preview box
                    widget::container(
                        widget::row()
                            .push(
                                widget::container(widget::Space::new(
                                    Length::Fixed(24.0),
                                    Length::Fixed(32.0),
                                ))
                                .class(
                                    cosmic::theme::Container::custom(move |_| {
                                        widget::container::Style {
                                            background: Some(cosmic::iced::Background::Color(
                                                background,
                                            )),
                                            ..Default::default()
                                        }
                                    }),
                                ),
                            )
                            .push(
                                widget::container(widget::Space::new(
                                    Length::Fixed(24.0),
                                    Length::Fixed(32.0),
                                ))
                                .class(
                                    cosmic::theme::Container::custom(move |_| {
                                        widget::container::Style {
                                            background: Some(cosmic::iced::Background::Color(
                                                accent,
                                            )),
                                            ..Default::default()
                                        }
                                    }),
                                ),
                            )
                            .push(
                                widget::container(widget::Space::new(
                                    Length::Fixed(24.0),
                                    Length::Fixed(32.0),
                                ))
                                .class(
                                    cosmic::theme::Container::custom(move |_| {
                                        widget::container::Style {
                                            background: Some(cosmic::iced::Background::Color(
                                                text_color,
                                            )),
                                            ..Default::default()
                                        }
                                    }),
                                ),
                            ),
                    )
                    .class(cosmic::theme::Container::custom(move |_| {
                        widget::container::Style {
                            border: cosmic::iced::Border {
                                radius: 4.0.into(),
                                width: if is_current { 2.0 } else { 1.0 },
                                color: if is_current {
                                    accent
                                } else {
                                    cosmic::iced::Color::from_rgba(0.5, 0.5, 0.5, 0.3)
                                },
                            },
                            ..Default::default()
                        }
                    })),
                )
                .push(widget::text::body(display_name));

            row = row.push(
                widget::button::custom(card)
                    .width(Length::Fixed(88.0))
                    .padding(spacing.space_xxs)
                    .on_press(Message::Page(pages::Message::Themes(
                        pages::ThemesMessage::SelectTheme(theme_id),
                    ))),
            );
        }

        row.into()
    }

    /// View for the Wallpapers page
    fn view_wallpapers_page(&self) -> Element<'_, Message> {
        let spacing = cosmic::theme::spacing();
        let cfg = &self.wallpaper_config;

        let mut column = widget::column()
            .spacing(spacing.space_m)
            .padding(spacing.space_m)
            .push(widget::text::title2(fl!("wallpapers")))
            .push(widget::text::body(fl!("wallpapers-description")));

        // Current wallpaper section
        column = column.push(
            widget::settings::section()
                .title("Current Wallpaper")
                .add(widget::settings::item(
                    "File",
                    widget::text::body(cfg.current_wallpaper_name()),
                ))
                .add(widget::settings::item(
                    "Theme",
                    widget::text::body(cfg.current_theme_name()),
                ))
                .add(widget::settings::item(
                    "Scaling",
                    widget::text::body(&cfg.scaling_mode),
                ))
                .add(widget::settings::item(
                    "Rotation",
                    widget::text::body(cfg.format_rotation()),
                )),
        );

        // Available themes section
        let theme_names = cfg.theme_names();
        let mut themes_section = widget::settings::section().title(format!(
            "Available Themes ({} themes, {} wallpapers)",
            theme_names.len(),
            cfg.total_wallpaper_count()
        ));

        for name in theme_names {
            if let Some(theme) = cfg.available_themes.get(&name) {
                themes_section = themes_section.add(widget::settings::item(
                    name,
                    widget::text::body(format!("{} wallpapers", theme.count)),
                ));
            }
        }

        column = column.push(themes_section);

        // Coming soon section
        column = column.push(
            widget::settings::section()
                .title("Coming Soon")
                .add(widget::settings::item(
                    "Wallpaper selection",
                    widget::text::body("Coming in Phase 3"),
                ))
                .add(widget::settings::item(
                    "Wallpaper import",
                    widget::text::body("Coming in Phase 3"),
                )),
        );

        column.into()
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
                        widget::text::body(ScreensaverConfig::format_timeout(
                            cfg.battery_idle_timeout,
                        )),
                    )),
            )
            .into()
    }
}
