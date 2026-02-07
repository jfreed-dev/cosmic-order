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
use crate::theme_config::{ThemeConfig, ThemePreviewState};
use crate::wallpaper_config::{ThumbnailCache, WallpaperConfig};

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
    /// Backup of theme state during preview (None when not previewing)
    theme_preview_backup: Option<ThemePreviewState>,
    /// Selected wallpaper collection filter
    wallpaper_selected_collection: Option<String>,
    /// Full path of the currently highlighted wallpaper in the grid
    wallpaper_selected_path: Option<String>,
    /// Current page offset in the wallpaper grid (for pagination)
    wallpaper_grid_page: usize,
    /// Thumbnail cache for wallpaper grid performance
    thumbnail_cache: ThumbnailCache,
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

        let initial_collection = wallpaper_config.current_theme_name();
        let app = Self {
            core,
            config,
            nav_model,
            active_page,
            screensaver_config,
            theme_config,
            wallpaper_config,
            theme_preview_backup: None,
            wallpaper_selected_collection: Some(initial_collection),
            wallpaper_selected_path: None,
            wallpaper_grid_page: 0,
            thumbnail_cache: ThumbnailCache::new(),
        };

        (app, Task::none())
    }

    fn nav_model(&self) -> Option<&nav_bar::Model> {
        Some(&self.nav_model)
    }

    #[allow(clippy::cognitive_complexity)]
    fn on_nav_select(&mut self, id: nav_bar::Id) -> Task<Message> {
        // Auto-cancel theme preview when navigating away
        if let Some(backup) = self.theme_preview_backup.take() {
            if let Err(e) = ThemeConfig::set_dark_mode(backup.config.is_dark) {
                tracing::error!("Failed to restore theme on nav: {e}");
            }
            self.theme_config = backup.config;
            tracing::info!("Theme preview auto-cancelled on navigation");
        }

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
            pages::Message::Wallpapers(msg) => self.handle_wallpapers_message(msg),
            pages::Message::Screensaver(_msg) => {
                // TODO: Implement screensaver message handling
                Task::none()
            }
        }
    }

    /// Handle theme page messages
    #[allow(clippy::cognitive_complexity)]
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
            pages::ThemesMessage::Export => {
                let default_name = crate::theme_config::ThemeConfig::default_export_filename();
                cosmic::task::future(async move {
                    let result = Self::run_theme_export(default_name).await;
                    Message::Page(pages::Message::Themes(
                        pages::ThemesMessage::ExportComplete(result),
                    ))
                })
            }
            pages::ThemesMessage::ExportComplete(result) => {
                match &result {
                    Ok(path) => tracing::info!("Theme exported to: {path}"),
                    Err(e) => {
                        if e != "cancelled" {
                            tracing::error!("Theme export failed: {e}");
                        } else {
                            tracing::debug!("Theme export cancelled by user");
                        }
                    }
                }
                Task::none()
            }
            pages::ThemesMessage::Import => {
                cosmic::task::future(async move {
                    let result = Self::run_theme_import().await;
                    Message::Page(pages::Message::Themes(
                        pages::ThemesMessage::ImportComplete(result),
                    ))
                })
            }
            pages::ThemesMessage::ImportComplete(result) => {
                match &result {
                    Ok(path) => {
                        tracing::info!("Theme imported from: {path}");
                        self.theme_config = ThemeConfig::load();
                    }
                    Err(e) => {
                        if e != "cancelled" {
                            tracing::error!("Theme import failed: {e}");
                        } else {
                            tracing::debug!("Theme import cancelled by user");
                        }
                    }
                }
                Task::none()
            }
            pages::ThemesMessage::PreviewTheme(theme_id) => {
                let previews = crate::theme_config::ThemePreview::built_in_themes();
                if let Some(preview) = previews.iter().find(|p| p.id == theme_id) {
                    // Only snapshot the original state if no preview is active yet
                    if self.theme_preview_backup.is_none() {
                        self.theme_preview_backup = Some(ThemePreviewState {
                            config: self.theme_config.clone(),
                            previewing_id: theme_id,
                        });
                    } else if let Some(ref mut backup) = self.theme_preview_backup {
                        // Switching between previews — keep original backup, update previewing id
                        backup.previewing_id = theme_id;
                    }

                    if let Err(e) = preview.apply() {
                        tracing::error!("Failed to preview theme: {e}");
                        // On failure, clear the backup
                        self.theme_preview_backup = None;
                    } else {
                        self.theme_config.is_dark = preview.is_dark;
                        self.theme_config.name = preview.name.clone();
                        tracing::info!("Previewing theme: {}", preview.name);
                    }
                }
                Task::none()
            }
            pages::ThemesMessage::ConfirmPreview => {
                if let Some(backup) = self.theme_preview_backup.take() {
                    tracing::info!(
                        "Confirmed preview — theme {:?} is now applied",
                        backup.previewing_id
                    );
                }
                Task::none()
            }
            pages::ThemesMessage::CancelPreview => {
                if let Some(backup) = self.theme_preview_backup.take() {
                    if let Err(e) = ThemeConfig::set_dark_mode(backup.config.is_dark) {
                        tracing::error!("Failed to restore theme: {e}");
                    } else {
                        self.theme_config = backup.config;
                        tracing::info!("Theme preview cancelled, restored previous theme");
                    }
                }
                Task::none()
            }
        }
    }

    /// Handle wallpaper page messages
    #[allow(clippy::cognitive_complexity)]
    fn handle_wallpapers_message(&mut self, message: pages::WallpapersMessage) -> Task<Message> {
        match message {
            pages::WallpapersMessage::SelectCollection(collection) => {
                self.wallpaper_selected_collection = collection;
                self.wallpaper_selected_path = None;
                self.wallpaper_grid_page = 0;
                Task::none()
            }
            pages::WallpapersMessage::SelectWallpaper(path) => {
                self.wallpaper_selected_path = Some(path);
                Task::none()
            }
            pages::WallpapersMessage::ApplyWallpaper => {
                let path = match &self.wallpaper_selected_path {
                    Some(p) => p.clone(),
                    None => return Task::none(),
                };
                let config_path = WallpaperConfig::config_path();
                cosmic::task::future(async move {
                    let result = Self::run_apply_wallpaper(path, config_path).await;
                    Message::Page(pages::Message::Wallpapers(
                        pages::WallpapersMessage::ApplyComplete(result),
                    ))
                })
            }
            pages::WallpapersMessage::ApplyComplete(result) => {
                match &result {
                    Ok(path) => {
                        self.wallpaper_config.current_source = path.clone();
                        tracing::info!("Wallpaper applied: {path}");
                    }
                    Err(e) => {
                        tracing::error!("Failed to apply wallpaper: {e}");
                    }
                }
                Task::none()
            }
            pages::WallpapersMessage::SetRotationEnabled(enabled) => {
                self.wallpaper_config.rotation_enabled = enabled;
                Task::none()
            }
            pages::WallpapersMessage::SetRotationFrequency(freq) => {
                self.wallpaper_config.rotation_frequency = freq;
                Task::none()
            }
            pages::WallpapersMessage::SetScalingMode(index) => {
                let modes = crate::wallpaper_config::ScalingMode::all();
                if let Some(mode) = modes.get(index) {
                    self.wallpaper_config.scaling_mode = mode.clone();
                }
                Task::none()
            }
            pages::WallpapersMessage::SaveSettings => {
                let config = self.wallpaper_config.clone();
                cosmic::task::future(async move {
                    let result = config.save().map_err(|e| e.to_string());
                    Message::Page(pages::Message::Wallpapers(
                        pages::WallpapersMessage::SaveComplete(result),
                    ))
                })
            }
            pages::WallpapersMessage::SaveComplete(result) => {
                match &result {
                    Ok(()) => tracing::info!("Wallpaper settings saved"),
                    Err(e) => tracing::error!("Failed to save wallpaper settings: {e}"),
                }
                Task::none()
            }
            pages::WallpapersMessage::ImportFromFile => {
                cosmic::task::future(async move {
                    let result = Self::run_wallpaper_import().await;
                    Message::Page(pages::Message::Wallpapers(
                        pages::WallpapersMessage::ImportComplete(result),
                    ))
                })
            }
            pages::WallpapersMessage::ImportComplete(result) => {
                match &result {
                    Ok(path) => {
                        tracing::info!("Wallpaper imported: {path}");
                        // Reload wallpaper config to pick up the new file
                        self.wallpaper_config = WallpaperConfig::load();
                    }
                    Err(e) => {
                        if e != "cancelled" {
                            tracing::error!("Wallpaper import failed: {e}");
                        } else {
                            tracing::debug!("Wallpaper import cancelled by user");
                        }
                    }
                }
                Task::none()
            }
            pages::WallpapersMessage::GridNextPage => {
                self.wallpaper_grid_page += 1;
                Task::none()
            }
            pages::WallpapersMessage::GridPrevPage => {
                self.wallpaper_grid_page = self.wallpaper_grid_page.saturating_sub(1);
                Task::none()
            }
        }
    }

    /// Apply a wallpaper: read current config, update source, write back
    async fn run_apply_wallpaper(
        path: String,
        config_path: std::path::PathBuf,
    ) -> Result<String, String> {
        // Read existing config or create default entry
        let content = tokio::fs::read_to_string(&config_path)
            .await
            .unwrap_or_default();

        let mut entry = ron::from_str::<crate::wallpaper_config::CosmicBgEntry>(&content)
            .unwrap_or_else(|_| crate::wallpaper_config::CosmicBgEntry {
                output: "all".to_string(),
                source: crate::wallpaper_config::BgSource::Path(String::new()),
                filter_by_theme: true,
                rotation_frequency: 600,
                filter_method: crate::wallpaper_config::FilterMethod::Lanczos,
                scaling_mode: crate::wallpaper_config::ScalingMode::Zoom,
                sampling_method: crate::wallpaper_config::SamplingMethod::Random,
            });

        entry.source = crate::wallpaper_config::BgSource::Path(path.clone());

        let pretty = ron::ser::PrettyConfig::default();
        let serialized = ron::ser::to_string_pretty(&entry, pretty)
            .map_err(|e| format!("Serialize error: {e}"))?;

        if let Some(parent) = config_path.parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .map_err(|e| format!("Create dir error: {e}"))?;
        }

        tokio::fs::write(&config_path, serialized)
            .await
            .map_err(|e| format!("Write error: {e}"))?;

        Ok(path)
    }

    /// Import a wallpaper file via xdg-portal file picker
    async fn run_wallpaper_import() -> Result<String, String> {
        use cosmic::dialog::file_chooser;

        let dialog = file_chooser::open::Dialog::new()
            .title(fl!("wallpaper-add-file"))
            .filter(
                file_chooser::FileFilter::new("Images")
                    .glob("*.png")
                    .glob("*.jpg")
                    .glob("*.jpeg")
                    .glob("*.webp")
                    .glob("*.gif")
                    .glob("*.bmp"),
            );

        let response = match dialog.open_file().await {
            Ok(r) => r,
            Err(file_chooser::Error::Cancelled) => return Err("cancelled".to_string()),
            Err(e) => return Err(format!("Dialog error: {e}")),
        };

        let url = response.url();
        let src_path = url
            .to_file_path()
            .map_err(|_| "Invalid file path".to_string())?;

        let dest_dir = WallpaperConfig::user_wallpapers_dir();
        tokio::fs::create_dir_all(&dest_dir)
            .await
            .map_err(|e| format!("Failed to create directory: {e}"))?;

        let filename = src_path
            .file_name()
            .ok_or_else(|| "No filename".to_string())?;
        let dest_path = dest_dir.join(filename);

        tokio::fs::copy(&src_path, &dest_path)
            .await
            .map_err(|e| format!("Failed to copy: {e}"))?;

        Ok(dest_path.to_string_lossy().to_string())
    }

    /// Run the theme export flow: open save dialog, serialize, write
    async fn run_theme_export(default_name: String) -> Result<String, String> {
        use cosmic::dialog::file_chooser;

        let dialog = file_chooser::save::Dialog::new()
            .title(fl!("theme-export"))
            .file_name(default_name)
            .filter(file_chooser::FileFilter::new("RON Theme").glob("*.ron"));

        let response = match dialog.save_file().await {
            Ok(r) => r,
            Err(file_chooser::Error::Cancelled) => return Err("cancelled".to_string()),
            Err(e) => return Err(format!("Dialog error: {e}")),
        };

        let url = response.url().ok_or_else(|| "No file URL returned".to_string())?;
        let path = url
            .to_file_path()
            .map_err(|_| "Invalid file path".to_string())?;

        crate::theme_config::ThemeConfig::export_theme(&path)
            .await
            .map_err(|e| e.to_string())
    }

    /// Run the theme import flow: open file dialog, read, deserialize, apply
    async fn run_theme_import() -> Result<String, String> {
        use cosmic::dialog::file_chooser;

        let dialog = file_chooser::open::Dialog::new()
            .title(fl!("theme-import"))
            .filter(file_chooser::FileFilter::new("RON Theme").glob("*.ron"));

        let response = match dialog.open_file().await {
            Ok(r) => r,
            Err(file_chooser::Error::Cancelled) => return Err("cancelled".to_string()),
            Err(e) => return Err(format!("Dialog error: {e}")),
        };

        let url = response.url();
        let path = url
            .to_file_path()
            .map_err(|_| "Invalid file path".to_string())?;

        crate::theme_config::ThemeConfig::import_theme(&path)
            .await
            .map_err(|e| e.to_string())
    }

    /// View for the Themes page
    fn view_themes_page(&self) -> Element<'_, Message> {
        let spacing = cosmic::theme::spacing();
        let cfg = &self.theme_config;

        let mut column = widget::column()
            .spacing(spacing.space_m)
            .padding(spacing.space_m)
            .push(widget::text::title2(fl!("themes")))
            .push(widget::text::body(fl!("themes-description")));

        // Insert preview banner when actively previewing
        if let Some(banner) = self.view_preview_banner() {
            column = column.push(banner);
        }

        column = column
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
            // Export & Import
            .push(
                widget::settings::section()
                    .title(fl!("theme-export-import"))
                    .add(
                        widget::column()
                            .spacing(spacing.space_xs)
                            .push(widget::text::body(fl!("theme-export-description")))
                            .push(
                                widget::button::standard(fl!("theme-export")).on_press(
                                    Message::Page(pages::Message::Themes(
                                        pages::ThemesMessage::Export,
                                    )),
                                ),
                            ),
                    )
                    .add(
                        widget::column()
                            .spacing(spacing.space_xs)
                            .push(widget::text::body(fl!("theme-import-description")))
                            .push(
                                widget::button::standard(fl!("theme-import")).on_press(
                                    Message::Page(pages::Message::Themes(
                                        pages::ThemesMessage::Import,
                                    )),
                                ),
                            ),
                    ),
            );

        column.into()
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

    /// Create theme list with mini-UI mockup cards and "Try" button
    fn view_theme_list(&self) -> Element<'_, Message> {
        use cosmic::iced::Length;
        let spacing = cosmic::theme::spacing();

        let is_previewing = self.theme_preview_backup.is_some();
        let previewing_id = self
            .theme_preview_backup
            .as_ref()
            .map(|b| b.previewing_id);

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
            let is_current = !is_previewing && self.theme_config.is_dark == preview.is_dark;
            let is_preview_active = previewing_id == Some(theme_id);
            let display_name = if preview.is_dark {
                fl!("theme-mode-dark")
            } else {
                fl!("theme-mode-light")
            };

            // Mini-UI mockup: background with accent bar and text-colored lines
            let mockup = widget::container(
                widget::column()
                    .spacing(4)
                    .padding(6)
                    // Accent bar at top (simulates a header/titlebar)
                    .push(
                        widget::container(widget::Space::new(
                            Length::Fill,
                            Length::Fixed(6.0),
                        ))
                        .class(cosmic::theme::Container::custom(move |_| {
                            widget::container::Style {
                                background: Some(cosmic::iced::Background::Color(accent)),
                                border: cosmic::iced::Border {
                                    radius: 2.0.into(),
                                    ..Default::default()
                                },
                                ..Default::default()
                            }
                        })),
                    )
                    // Text line 1 (wider)
                    .push(
                        widget::container(widget::Space::new(
                            Length::Fixed(80.0),
                            Length::Fixed(4.0),
                        ))
                        .class(cosmic::theme::Container::custom(move |_| {
                            widget::container::Style {
                                background: Some(cosmic::iced::Background::Color(
                                    cosmic::iced::Color {
                                        a: 0.7,
                                        ..text_color
                                    },
                                )),
                                border: cosmic::iced::Border {
                                    radius: 1.0.into(),
                                    ..Default::default()
                                },
                                ..Default::default()
                            }
                        })),
                    )
                    // Text line 2 (shorter)
                    .push(
                        widget::container(widget::Space::new(
                            Length::Fixed(56.0),
                            Length::Fixed(4.0),
                        ))
                        .class(cosmic::theme::Container::custom(move |_| {
                            widget::container::Style {
                                background: Some(cosmic::iced::Background::Color(
                                    cosmic::iced::Color {
                                        a: 0.5,
                                        ..text_color
                                    },
                                )),
                                border: cosmic::iced::Border {
                                    radius: 1.0.into(),
                                    ..Default::default()
                                },
                                ..Default::default()
                            }
                        })),
                    )
                    // Text line 3 (medium)
                    .push(
                        widget::container(widget::Space::new(
                            Length::Fixed(66.0),
                            Length::Fixed(4.0),
                        ))
                        .class(cosmic::theme::Container::custom(move |_| {
                            widget::container::Style {
                                background: Some(cosmic::iced::Background::Color(
                                    cosmic::iced::Color {
                                        a: 0.4,
                                        ..text_color
                                    },
                                )),
                                border: cosmic::iced::Border {
                                    radius: 1.0.into(),
                                    ..Default::default()
                                },
                                ..Default::default()
                            }
                        })),
                    )
                    // Small accent button mockup at bottom
                    .push(
                        widget::container(widget::Space::new(
                            Length::Fixed(36.0),
                            Length::Fixed(8.0),
                        ))
                        .class(cosmic::theme::Container::custom(move |_| {
                            widget::container::Style {
                                background: Some(cosmic::iced::Background::Color(
                                    cosmic::iced::Color {
                                        a: 0.8,
                                        ..accent
                                    },
                                )),
                                border: cosmic::iced::Border {
                                    radius: 3.0.into(),
                                    ..Default::default()
                                },
                                ..Default::default()
                            }
                        })),
                    ),
            )
            .width(Length::Fixed(120.0))
            .height(Length::Fixed(80.0))
            .class(cosmic::theme::Container::custom(move |_| {
                let border_color = if is_preview_active {
                    // Gold border for actively previewed theme
                    cosmic::iced::Color::from_rgb(0.85, 0.65, 0.13)
                } else if is_current {
                    accent
                } else {
                    cosmic::iced::Color::from_rgba(0.5, 0.5, 0.5, 0.3)
                };
                widget::container::Style {
                    background: Some(cosmic::iced::Background::Color(background)),
                    border: cosmic::iced::Border {
                        radius: 6.0.into(),
                        width: if is_current || is_preview_active {
                            2.0
                        } else {
                            1.0
                        },
                        color: border_color,
                    },
                    ..Default::default()
                }
            }));

            // Card: mockup + name + Try button
            let card = widget::column()
                .spacing(spacing.space_xxs)
                .width(Length::Fixed(140.0))
                .align_x(cosmic::iced::Alignment::Center)
                .push(mockup)
                .push(widget::text::body(display_name))
                .push(
                    widget::button::standard(fl!("theme-try")).on_press(Message::Page(
                        pages::Message::Themes(pages::ThemesMessage::PreviewTheme(theme_id)),
                    )),
                );

            row = row.push(card);
        }

        row.into()
    }

    /// Build the preview confirmation banner (shown when a theme is being previewed)
    fn view_preview_banner(&self) -> Option<Element<'_, Message>> {
        self.theme_preview_backup.as_ref()?;

        let spacing = cosmic::theme::spacing();

        let banner = widget::container(
            widget::row()
                .spacing(spacing.space_m)
                .align_y(cosmic::iced::Alignment::Center)
                .push(
                    widget::text::body(fl!("theme-preview-active"))
                        .width(cosmic::iced::Length::Fill),
                )
                .push(
                    widget::button::standard(fl!("cancel")).on_press(Message::Page(
                        pages::Message::Themes(pages::ThemesMessage::CancelPreview),
                    )),
                )
                .push(
                    widget::button::suggested(fl!("theme-apply")).on_press(Message::Page(
                        pages::Message::Themes(pages::ThemesMessage::ConfirmPreview),
                    )),
                ),
        )
        .padding(spacing.space_s)
        .class(cosmic::theme::Container::custom(|theme| {
            let accent = theme.cosmic().accent.base;
            widget::container::Style {
                background: Some(cosmic::iced::Background::Color(cosmic::iced::Color::from_rgba(
                    accent.red,
                    accent.green,
                    accent.blue,
                    0.15,
                ))),
                border: cosmic::iced::Border {
                    radius: 8.0.into(),
                    width: 1.0,
                    color: cosmic::iced::Color::from_rgba(
                        accent.red,
                        accent.green,
                        accent.blue,
                        0.4,
                    ),
                },
                ..Default::default()
            }
        }));

        Some(banner.into())
    }

    /// View for the Wallpapers page
    fn view_wallpapers_page(&self) -> Element<'_, Message> {
        let spacing = cosmic::theme::spacing();

        let column = widget::column()
            .spacing(spacing.space_m)
            .padding(spacing.space_m)
            .push(widget::text::title2(fl!("wallpapers")))
            .push(widget::text::body(fl!("wallpapers-description")))
            .push(self.view_wallpaper_current_section())
            .push(self.view_wallpaper_collection_selector())
            .push(self.view_wallpaper_grid())
            .push(self.view_wallpaper_rotation_section());

        widget::scrollable(column).into()
    }

    /// Current wallpaper info + Apply and Import buttons
    fn view_wallpaper_current_section(&self) -> Element<'_, Message> {
        let spacing = cosmic::theme::spacing();
        let cfg = &self.wallpaper_config;

        let has_selection = self.wallpaper_selected_path.is_some();

        let mut section = widget::settings::section()
            .title(fl!("wallpaper-current"))
            .add(widget::settings::item(
                fl!("wallpaper-file"),
                widget::text::body(cfg.current_wallpaper_name()),
            ))
            .add(widget::settings::item(
                fl!("wallpaper-theme"),
                widget::text::body(cfg.current_theme_name()),
            ));

        // Action buttons row
        let mut buttons_row = widget::row().spacing(spacing.space_s);

        let apply_button = if has_selection {
            widget::button::suggested(fl!("wallpaper-set")).on_press(Message::Page(
                pages::Message::Wallpapers(pages::WallpapersMessage::ApplyWallpaper),
            ))
        } else {
            widget::button::suggested(fl!("wallpaper-set"))
        };

        let import_button =
            widget::button::standard(fl!("wallpaper-add-file")).on_press(Message::Page(
                pages::Message::Wallpapers(pages::WallpapersMessage::ImportFromFile),
            ));

        buttons_row = buttons_row.push(apply_button).push(import_button);

        section = section.add(buttons_row);

        section.into()
    }

    /// Collection selector dropdown
    fn view_wallpaper_collection_selector(&self) -> Element<'_, Message> {
        let cfg = &self.wallpaper_config;
        let theme_names = cfg.theme_names();

        // Build options: sorted theme names only (no "All" — too many images to render)
        let options: Vec<String> = theme_names;

        // Determine selected index
        let selected = self
            .wallpaper_selected_collection
            .as_ref()
            .and_then(|name| options.iter().position(|o| o == name));

        let options_for_closure = options.clone();
        let dropdown = widget::dropdown(options, selected, move |index| {
            let collection = options_for_closure.get(index).cloned();
            Message::Page(pages::Message::Wallpapers(
                pages::WallpapersMessage::SelectCollection(collection),
            ))
        });

        widget::settings::section()
            .title(fl!("wallpaper-collection"))
            .add(widget::settings::item(
                fl!("wallpaper-collection"),
                dropdown,
            ))
            .into()
    }

    /// Wallpaper thumbnail grid with pagination
    fn view_wallpaper_grid(&self) -> Element<'_, Message> {
        use cosmic::iced::Length;

        const PER_PAGE: usize = 12;
        let spacing = cosmic::theme::spacing();
        let cfg = &self.wallpaper_config;

        // Collect wallpapers for the selected collection
        let wallpapers: Vec<(String, String)> =
            if let Some(theme) = self.wallpaper_selected_collection.as_ref().and_then(|c| cfg.available_themes.get(c)) {
                theme
                    .wallpapers
                    .iter()
                    .map(|filename| {
                        let full = theme.path.join(filename);
                        (full.to_string_lossy().to_string(), filename.clone())
                    })
                    .collect()
            } else {
                Vec::new()
            };

        if wallpapers.is_empty() {
            return widget::container(widget::text::body(fl!("wallpaper-no-wallpapers")))
                .padding(spacing.space_l)
                .width(Length::Fill)
                .into();
        }

        let total = wallpapers.len();
        let total_pages = total.div_ceil(PER_PAGE);
        let page = self.wallpaper_grid_page.min(total_pages.saturating_sub(1));
        let start = page * PER_PAGE;
        let end = (start + PER_PAGE).min(total);

        let cards: Vec<Element<'_, Message>> = wallpapers[start..end]
            .iter()
            .map(|(full_path, filename)| self.view_wallpaper_card(full_path, filename))
            .collect();

        let grid = widget::flex_row(cards)
            .column_spacing(spacing.space_s)
            .row_spacing(spacing.space_s)
            .width(Length::Fill);

        // Pagination controls (only if more than one page)
        if total_pages <= 1 {
            return grid.into();
        }

        let mut nav_row = widget::row()
            .spacing(spacing.space_s)
            .align_y(cosmic::iced::Alignment::Center);

        if page > 0 {
            nav_row = nav_row.push(
                widget::button::standard("<").on_press(Message::Page(
                    pages::Message::Wallpapers(pages::WallpapersMessage::GridPrevPage),
                )),
            );
        } else {
            nav_row = nav_row.push(widget::button::standard("<"));
        }

        nav_row = nav_row.push(widget::text::body(format!(
            "{} / {}",
            page + 1,
            total_pages,
        )));

        if page + 1 < total_pages {
            nav_row = nav_row.push(
                widget::button::standard(">").on_press(Message::Page(
                    pages::Message::Wallpapers(pages::WallpapersMessage::GridNextPage),
                )),
            );
        } else {
            nav_row = nav_row.push(widget::button::standard(">"));
        }

        widget::column()
            .spacing(spacing.space_s)
            .push(grid)
            .push(nav_row)
            .into()
    }

    /// Single wallpaper thumbnail card
    fn view_wallpaper_card<'a>(
        &'a self,
        full_path: &str,
        filename: &str,
    ) -> Element<'a, Message> {
        use cosmic::iced::Length;
        use cosmic::widget::image::Handle;

        let spacing = cosmic::theme::spacing();
        let is_current = self.wallpaper_config.current_source == full_path;
        let is_selected = self
            .wallpaper_selected_path
            .as_deref()
            == Some(full_path);

        let path_owned = full_path.to_string();

        let thumb_path = self.thumbnail_cache.get_or_create(full_path);
        let image_button = widget::button::image(Handle::from_path(thumb_path))
            .width(Length::Fixed(160.0))
            .height(Length::Fixed(100.0))
            .selected(is_current || is_selected)
            .on_press(Message::Page(pages::Message::Wallpapers(
                pages::WallpapersMessage::SelectWallpaper(path_owned),
            )));

        // Truncate filename for display
        let display_name = if filename.len() > 20 {
            format!("{}...", &filename[..17])
        } else {
            filename.to_string()
        };

        widget::column()
            .spacing(spacing.space_xxs)
            .align_x(cosmic::iced::Alignment::Center)
            .width(Length::Fixed(168.0))
            .push(image_button)
            .push(widget::text::caption(display_name))
            .into()
    }

    /// Rotation and scaling settings section
    fn view_wallpaper_rotation_section(&self) -> Element<'_, Message> {
        let spacing = cosmic::theme::spacing();
        let cfg = &self.wallpaper_config;

        // Rotation toggle
        let rotation_toggle = widget::toggler(cfg.rotation_enabled).on_toggle(|enabled| {
            Message::Page(pages::Message::Wallpapers(
                pages::WallpapersMessage::SetRotationEnabled(enabled),
            ))
        });

        // Rotation frequency options
        let freq_options: Vec<String> = vec![
            fl!("wallpaper-5min"),
            fl!("wallpaper-10min"),
            fl!("wallpaper-15min"),
            fl!("wallpaper-30min"),
            fl!("wallpaper-1hour"),
        ];
        let freq_values: [u32; 5] = [300, 600, 900, 1800, 3600];
        let freq_selected = freq_values
            .iter()
            .position(|&v| v == cfg.rotation_frequency);

        let freq_dropdown = widget::dropdown(freq_options, freq_selected, move |index| {
            let freq = freq_values.get(index).copied().unwrap_or(600);
            Message::Page(pages::Message::Wallpapers(
                pages::WallpapersMessage::SetRotationFrequency(freq),
            ))
        });

        // Scaling mode options
        let scaling_options: Vec<String> = crate::wallpaper_config::ScalingMode::all()
            .iter()
            .map(|m| m.to_string())
            .collect();
        let scaling_selected = crate::wallpaper_config::ScalingMode::all()
            .iter()
            .position(|m| *m == cfg.scaling_mode);

        let scaling_dropdown =
            widget::dropdown(scaling_options, scaling_selected, |index| {
                Message::Page(pages::Message::Wallpapers(
                    pages::WallpapersMessage::SetScalingMode(index),
                ))
            });

        // Save button
        let save_button = widget::button::suggested(fl!("save")).on_press(Message::Page(
            pages::Message::Wallpapers(pages::WallpapersMessage::SaveSettings),
        ));

        widget::settings::section()
            .title(fl!("wallpaper-rotation"))
            .add(widget::settings::item(
                fl!("wallpaper-rotation-enabled"),
                rotation_toggle,
            ))
            .add(widget::settings::item(
                fl!("wallpaper-rotation-interval"),
                freq_dropdown,
            ))
            .add(widget::settings::item(
                fl!("wallpaper-scaling"),
                scaling_dropdown,
            ))
            .add(
                widget::row()
                    .spacing(spacing.space_s)
                    .push(save_button),
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
                        widget::text::body(ScreensaverConfig::format_timeout(
                            cfg.battery_idle_timeout,
                        )),
                    )),
            )
            .into()
    }
}
