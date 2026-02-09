// SPDX-License-Identifier: GPL-3.0-only

//! Main application module
//!
//! Implements the COSMIC Application trait and handles message routing.

use cosmic::app::{Core, Task};
use cosmic::widget::{self, nav_bar};
use cosmic::{Application, Element};

use crate::compositor;
use crate::config::Config;
use crate::fl;
use crate::inhibit;
use crate::pages::{self, PageId};
use crate::power;
use crate::screensaver_config::ScreensaverConfig;
use crate::sleep_lock;
use crate::theme_config::{ThemeConfig, ThemePreviewState};
use crate::tool_sync::ToolSyncConfig;
use crate::wallpaper_config::{ThumbnailCache, WallpaperConfig};
use crate::wayland_idle;
use std::path::PathBuf;

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
    /// Live power state from D-Bus (None until first update)
    power_state: Option<power::PowerState>,
    /// Available logos scanned from the logos directory
    available_logos: Vec<(String, PathBuf)>,
    /// Status message shown after save/reload (cleared on next action)
    screensaver_status_msg: Option<String>,
    /// Saved compositor settings during screensaver test (None when not testing)
    compositor_backup: Option<compositor::CompositorBackup>,
    /// Active idle inhibitor (Some = caffeine mode on)
    caffeine_inhibitor: Option<inhibit::IdleInhibitor>,
    /// Whether caffeine mode is active (mirrors inhibitor presence, needed for view)
    caffeine_active: bool,
    /// Tool sync configuration (which tools to sync)
    tool_sync_config: ToolSyncConfig,
    /// Status message from last sync operation
    tool_sync_status: Option<String>,
    /// Whether native Wayland idle detection is active (swayidle stopped)
    native_idle_active: bool,
    /// Configuration for the idle subscription (changes trigger restart)
    idle_subscription_config: wayland_idle::IdleSubscriptionConfig,
    /// Handle for the running screensaver child process
    idle_screensaver_child: Option<u32>,
}

/// Application messages
#[derive(Debug, Clone)]
#[allow(dead_code)] // Variants constructed by libcosmic framework
pub enum Message {
    /// Navigation item selected
    NavSelect(nav_bar::Id),
    /// Page-specific message
    Page(pages::Message),
    /// Configuration changed
    ConfigChanged(Config),
    /// Power state updated from D-Bus subscription
    PowerStateUpdate(power::PowerState),
    /// Toggle caffeine mode (idle inhibitor)
    ToggleCaffeine,
    /// Result of acquiring the idle inhibitor
    CaffeineResult(Result<inhibit::IdleInhibitor, String>),
    /// Wayland idle notification event
    IdleEvent(wayland_idle::IdleEvent),
    /// Logind sleep event (`PrepareForSleep`)
    SleepEvent(sleep_lock::SleepEvent),
    /// Restart swayidle fallback service
    RestartSwayidle,
    /// No-op (used by fire-and-forget async tasks)
    None,
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

    fn system_theme_update(
        &mut self,
        _keys: &[&'static str],
        _new_theme: &cosmic::cosmic_theme::Theme,
    ) -> Task<Message> {
        if self.tool_sync_config.auto_sync {
            return self.update(Message::Page(pages::Message::Themes(
                pages::ThemesMessage::SyncTools,
            )));
        }
        Task::none()
    }

    fn system_theme_mode_update(
        &mut self,
        _keys: &[&'static str],
        _new_theme: &cosmic::cosmic_theme::ThemeMode,
    ) -> Task<Message> {
        if self.tool_sync_config.auto_sync {
            return self.update(Message::Page(pages::Message::Themes(
                pages::ThemesMessage::SyncTools,
            )));
        }
        Task::none()
    }

    #[allow(clippy::cognitive_complexity)]
    fn init(core: Core, _flags: Self::Flags) -> (Self, Task<Message>) {
        // Load configuration
        let config = Config::load().unwrap_or_default();

        // Load screensaver configuration
        let mut screensaver_config = ScreensaverConfig::load().unwrap_or_default();

        // Override DPMS timeout from system config if available
        if let Some(system_dpms) = crate::cosmic_idle::read_screen_off_time() {
            screensaver_config.dpms_timeout = system_dpms;
        }

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

        // Scan available logos
        let available_logos = ScreensaverConfig::scan_logos();
        tracing::info!("Found {} available logos", available_logos.len());

        let idle_subscription_config = Self::compute_idle_config(&screensaver_config);
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
            power_state: None,
            available_logos,
            screensaver_status_msg: None,
            compositor_backup: None,
            caffeine_inhibitor: None,
            caffeine_active: false,
            tool_sync_config: ToolSyncConfig::load(),
            tool_sync_status: None,
            native_idle_active: false,
            idle_subscription_config,
            idle_screensaver_child: None,
        };

        let init_task = app.spawn_thumbnail_generation();
        (app, init_task)
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

    fn subscription(&self) -> cosmic::iced::Subscription<Self::Message> {
        cosmic::iced::Subscription::batch([
            power::power_subscription().map(Message::PowerStateUpdate),
            wayland_idle::idle_subscription(self.idle_subscription_config.clone())
                .map(Message::IdleEvent),
            sleep_lock::sleep_lock_subscription().map(Message::SleepEvent),
        ])
    }

    fn on_app_exit(&mut self) -> Option<Self::Message> {
        if self.native_idle_active {
            self.kill_idle_screensaver();
            Some(Message::RestartSwayidle)
        } else {
            None
        }
    }

    #[allow(clippy::cognitive_complexity)]
    fn update(&mut self, message: Self::Message) -> Task<Message> {
        match message {
            Message::NavSelect(id) => self.on_nav_select(id),
            Message::Page(page_message) => self.handle_page_message(page_message),
            Message::ConfigChanged(config) => {
                self.config = config;
                Task::none()
            }
            Message::PowerStateUpdate(state) => {
                // Write power-state.env for screensaver-ctl (fire-and-forget)
                let env_content = state.to_env_format();
                let env_path = ScreensaverConfig::power_env_path();
                tokio::spawn(async move {
                    if let Some(parent) = env_path.parent() {
                        let _ = tokio::fs::create_dir_all(parent).await;
                    }
                    if let Err(e) = tokio::fs::write(&env_path, env_content).await {
                        tracing::warn!("Failed to write power-state.env: {e}");
                    }
                });

                // Auto-disable caffeine on low battery
                if self.caffeine_active
                    && state.on_battery
                    && state.battery_percent.is_some_and(|p| p < 20)
                {
                    self.caffeine_inhibitor = None;
                    self.caffeine_active = false;
                    tracing::info!("{}", fl!("caffeine-disabled-battery"));
                }

                self.power_state = Some(state);
                Task::none()
            }
            Message::ToggleCaffeine => {
                if self.caffeine_active {
                    self.caffeine_inhibitor = None;
                    self.caffeine_active = false;
                    tracing::info!("Caffeine mode disabled");
                    Task::none()
                } else {
                    cosmic::task::future(async {
                        let result = inhibit::IdleInhibitor::acquire().await;
                        Message::CaffeineResult(result)
                    })
                }
            }
            Message::CaffeineResult(result) => {
                match result {
                    Ok(inhibitor) => {
                        self.caffeine_inhibitor = Some(inhibitor);
                        self.caffeine_active = true;
                        tracing::info!("{}", fl!("caffeine-enabled"));
                    }
                    Err(e) => {
                        self.caffeine_active = false;
                        tracing::error!("Failed to acquire idle inhibitor: {e}");
                    }
                }
                Task::none()
            }
            Message::IdleEvent(event) => self.handle_idle_event(event),
            Message::SleepEvent(event) => self.handle_sleep_event(event),
            Message::RestartSwayidle => {
                Self::restart_swayidle_sync();
                Task::none()
            }
            Message::None => Task::none(),
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
        let caffeine_icon = widget::icon::from_name("preferences-system-power-symbolic");
        let tooltip_text = if self.caffeine_active {
            fl!("caffeine-enabled")
        } else {
            fl!("caffeine-toggle")
        };
        let caffeine_button = widget::button::icon(caffeine_icon)
            .selected(self.caffeine_active)
            .on_press(Message::ToggleCaffeine)
            .tooltip(tooltip_text);
        vec![caffeine_button.into()]
    }
}

impl App {
    /// Handle page-specific messages
    fn handle_page_message(&mut self, message: pages::Message) -> Task<Message> {
        match message {
            pages::Message::Themes(msg) => self.handle_themes_message(msg),
            pages::Message::Wallpapers(msg) => self.handle_wallpapers_message(msg),
            pages::Message::Screensaver(msg) => self.handle_screensaver_message(msg),
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
                        fl!("theme-mode-dark")
                    } else {
                        fl!("theme-mode-light")
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
            pages::ThemesMessage::Import => cosmic::task::future(async move {
                let result = Self::run_theme_import().await;
                Message::Page(pages::Message::Themes(
                    pages::ThemesMessage::ImportComplete(result),
                ))
            }),
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
            pages::ThemesMessage::SetGhosttySync(enabled) => {
                self.tool_sync_config.ghostty_enabled = enabled;
                let config = self.tool_sync_config.clone();
                cosmic::task::future(async move {
                    if let Err(e) = config.save().await {
                        tracing::warn!("Failed to save tool sync config: {e}");
                    }
                    Message::Page(pages::Message::Themes(pages::ThemesMessage::SyncComplete(
                        Ok(String::new()),
                    )))
                })
            }
            pages::ThemesMessage::SetBtopSync(enabled) => {
                self.tool_sync_config.btop_enabled = enabled;
                let config = self.tool_sync_config.clone();
                cosmic::task::future(async move {
                    if let Err(e) = config.save().await {
                        tracing::warn!("Failed to save tool sync config: {e}");
                    }
                    Message::Page(pages::Message::Themes(pages::ThemesMessage::SyncComplete(
                        Ok(String::new()),
                    )))
                })
            }
            pages::ThemesMessage::SetNvimSync(enabled) => {
                self.tool_sync_config.nvim_enabled = enabled;
                let config = self.tool_sync_config.clone();
                cosmic::task::future(async move {
                    if let Err(e) = config.save().await {
                        tracing::warn!("Failed to save tool sync config: {e}");
                    }
                    Message::Page(pages::Message::Themes(pages::ThemesMessage::SyncComplete(
                        Ok(String::new()),
                    )))
                })
            }
            pages::ThemesMessage::SetZellijSync(enabled) => {
                self.tool_sync_config.zellij_enabled = enabled;
                let config = self.tool_sync_config.clone();
                cosmic::task::future(async move {
                    if let Err(e) = config.save().await {
                        tracing::warn!("Failed to save tool sync config: {e}");
                    }
                    Message::Page(pages::Message::Themes(pages::ThemesMessage::SyncComplete(
                        Ok(String::new()),
                    )))
                })
            }
            pages::ThemesMessage::SetFzfSync(enabled) => {
                self.tool_sync_config.fzf_enabled = enabled;
                let config = self.tool_sync_config.clone();
                cosmic::task::future(async move {
                    if let Err(e) = config.save().await {
                        tracing::warn!("Failed to save tool sync config: {e}");
                    }
                    Message::Page(pages::Message::Themes(pages::ThemesMessage::SyncComplete(
                        Ok(String::new()),
                    )))
                })
            }
            pages::ThemesMessage::SetFzfShellIntegration(enabled) => {
                self.tool_sync_config.fzf_shell_integration = enabled;
                let config = self.tool_sync_config.clone();
                cosmic::task::future(async move {
                    if let Err(e) = config.save().await {
                        tracing::warn!("Failed to save tool sync config: {e}");
                    }
                    let shell_result = if enabled {
                        crate::generators::fzf::enable_shell_integration().await
                    } else {
                        crate::generators::fzf::disable_shell_integration().await
                    };
                    if let Err(e) = shell_result {
                        tracing::warn!("Failed to update fzf shell integration: {e}");
                    }
                    Message::Page(pages::Message::Themes(pages::ThemesMessage::SyncComplete(
                        Ok(String::new()),
                    )))
                })
            }
            pages::ThemesMessage::SetLazygitSync(enabled) => {
                self.tool_sync_config.lazygit_enabled = enabled;
                let config = self.tool_sync_config.clone();
                cosmic::task::future(async move {
                    if let Err(e) = config.save().await {
                        tracing::warn!("Failed to save tool sync config: {e}");
                    }
                    Message::Page(pages::Message::Themes(pages::ThemesMessage::SyncComplete(
                        Ok(String::new()),
                    )))
                })
            }
            pages::ThemesMessage::SetHooksEnabled(enabled) => {
                self.tool_sync_config.hooks_enabled = enabled;
                let config = self.tool_sync_config.clone();
                cosmic::task::future(async move {
                    if let Err(e) = config.save().await {
                        tracing::warn!("Failed to save tool sync config: {e}");
                    }
                    Message::Page(pages::Message::Themes(pages::ThemesMessage::SyncComplete(
                        Ok(String::new()),
                    )))
                })
            }
            pages::ThemesMessage::SetAutoSync(enabled) => {
                self.tool_sync_config.auto_sync = enabled;
                let config = self.tool_sync_config.clone();
                cosmic::task::future(async move {
                    if let Err(e) = config.save().await {
                        tracing::warn!("Failed to save tool sync config: {e}");
                    }
                    Message::Page(pages::Message::Themes(pages::ThemesMessage::SyncComplete(
                        Ok(String::new()),
                    )))
                })
            }
            pages::ThemesMessage::SyncTools => {
                self.tool_sync_status = None;
                let config = self.tool_sync_config.clone();
                cosmic::task::future(async move {
                    let result = crate::tool_sync::sync_tools(&config).await;
                    let msg = match result {
                        Ok(r) => {
                            let live = crate::tool_sync::signal_running_apps(&config).await;

                            let mut parts =
                                vec![format!("colors.toml: {}", r.colors_path.display())];
                            if r.ghostty_synced {
                                parts.push("Ghostty: synced".to_string());
                            }
                            if r.btop_synced {
                                parts.push("btop: synced".to_string());
                            }
                            if r.nvim_synced {
                                parts.push("Neovim: synced".to_string());
                            }
                            if r.zellij_synced {
                                parts.push("Zellij: synced".to_string());
                            }
                            if r.fzf_synced {
                                parts.push("fzf: synced".to_string());
                            }
                            if r.lazygit_synced {
                                parts.push("lazygit: synced".to_string());
                            }
                            if let Some(ref hr) = r.hooks_result {
                                if hr.hooks_run > 0 {
                                    parts.push(format!(
                                        "hooks: {}/{} ok",
                                        hr.hooks_succeeded, hr.hooks_run
                                    ));
                                }
                            }
                            if !live.is_empty() {
                                parts.push(format!("live: {}", live.join(", ")));
                            }
                            Ok(parts.join(", "))
                        }
                        Err(e) => Err(e),
                    };
                    Message::Page(pages::Message::Themes(pages::ThemesMessage::SyncComplete(
                        msg,
                    )))
                })
            }
            pages::ThemesMessage::SyncComplete(result) => {
                match &result {
                    Ok(summary) => {
                        if !summary.is_empty() {
                            tracing::info!("Tool sync complete: {summary}");
                            self.tool_sync_status = Some(fl!("tool-sync-success"));
                        }
                    }
                    Err(e) => {
                        tracing::error!("Tool sync failed: {e}");
                        self.tool_sync_status = Some(fl!("tool-sync-error"));
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
                self.spawn_thumbnail_generation()
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
            pages::WallpapersMessage::ImportFromFile => cosmic::task::future(async move {
                let result = Self::run_wallpaper_import().await;
                Message::Page(pages::Message::Wallpapers(
                    pages::WallpapersMessage::ImportComplete(result),
                ))
            }),
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
                self.spawn_thumbnail_generation()
            }
            pages::WallpapersMessage::GridPrevPage => {
                self.wallpaper_grid_page = self.wallpaper_grid_page.saturating_sub(1);
                self.spawn_thumbnail_generation()
            }
            pages::WallpapersMessage::ThumbnailsReady => {
                // Re-render will pick up newly cached thumbnails
                Task::none()
            }
        }
    }

    /// Handle screensaver page messages
    #[allow(clippy::cognitive_complexity)]
    fn handle_screensaver_message(&mut self, message: pages::ScreensaverMessage) -> Task<Message> {
        match message {
            pages::ScreensaverMessage::SetEnabled(enabled) => {
                self.screensaver_config.enabled = enabled;
                Task::none()
            }
            pages::ScreensaverMessage::SetIdleTimeout(seconds) => {
                self.screensaver_config.idle_timeout = seconds;
                // Bump lock and dpms up if they're non-zero but less than new idle
                let cfg = &mut self.screensaver_config;
                if cfg.lock_timeout > 0 && cfg.lock_timeout < seconds {
                    cfg.lock_timeout = seconds;
                }
                if cfg.dpms_timeout > 0 && cfg.dpms_timeout < seconds {
                    cfg.dpms_timeout = seconds;
                }
                Task::none()
            }
            pages::ScreensaverMessage::SetLockTimeout(seconds) => {
                // Lock must be >= idle, or 0 (disabled)
                let idle = self.screensaver_config.idle_timeout;
                self.screensaver_config.lock_timeout = if seconds > 0 && seconds < idle {
                    idle
                } else {
                    seconds
                };
                Task::none()
            }
            pages::ScreensaverMessage::SetDpmsTimeout(seconds) => {
                // Screen off must be >= idle, or 0 (disabled)
                let idle = self.screensaver_config.idle_timeout;
                self.screensaver_config.dpms_timeout = if seconds > 0 && seconds < idle {
                    idle
                } else {
                    seconds
                };
                Task::none()
            }
            pages::ScreensaverMessage::SetFrameRate(index) => {
                let rates: [u32; 3] = [30, 60, 120];
                if let Some(&rate) = rates.get(index) {
                    self.screensaver_config.frame_rate = rate;
                }
                Task::none()
            }
            pages::ScreensaverMessage::SetExcludeEffects(text) => {
                self.screensaver_config.exclude_effects = text;
                Task::none()
            }
            pages::ScreensaverMessage::SetIncludeEffects(text) => {
                self.screensaver_config.include_effects = text;
                Task::none()
            }
            pages::ScreensaverMessage::SetFadeInEffect(index) => {
                let effects = Self::fade_effect_values();
                if let Some(effect) = effects.get(index) {
                    self.screensaver_config.fade_in_effect = effect.clone();
                }
                Task::none()
            }
            pages::ScreensaverMessage::SetFadeOutEffect(index) => {
                let effects = Self::fade_effect_values();
                if let Some(effect) = effects.get(index) {
                    self.screensaver_config.fade_out_effect = effect.clone();
                }
                Task::none()
            }
            pages::ScreensaverMessage::SetShowClock(enabled) => {
                self.screensaver_config.show_clock = enabled;
                Task::none()
            }
            pages::ScreensaverMessage::SetClockDuration(index) => {
                let durations: [u32; 4] = [3, 5, 10, 15];
                if let Some(&dur) = durations.get(index) {
                    self.screensaver_config.clock_duration = dur;
                }
                Task::none()
            }
            pages::ScreensaverMessage::SetClockFormat(index) => {
                let formats = ["%H:%M", "%H:%M:%S", "%I:%M %p", "%I:%M:%S %p"];
                if let Some(&fmt) = formats.get(index) {
                    self.screensaver_config.clock_format = fmt.to_string();
                }
                Task::none()
            }
            pages::ScreensaverMessage::SetTerminal(index) => {
                let terminals = ["ghostty", "cosmic-term"];
                if let Some(&term) = terminals.get(index) {
                    self.screensaver_config.terminal = term.to_string();
                }
                Task::none()
            }
            pages::ScreensaverMessage::SetCursorHide(enabled) => {
                self.screensaver_config.cursor_hide = enabled;
                Task::none()
            }
            pages::ScreensaverMessage::SetHideMouse(enabled) => {
                self.screensaver_config.hide_mouse = enabled;
                Task::none()
            }
            pages::ScreensaverMessage::SetDismissOnKey(enabled) => {
                self.screensaver_config.dismiss_on_key = enabled;
                Task::none()
            }
            pages::ScreensaverMessage::SelectLogo(path) => {
                self.screensaver_config.logo_file = path;
                Task::none()
            }
            pages::ScreensaverMessage::SelectLogoDialog => cosmic::task::future(async move {
                let result = Self::run_logo_select().await;
                Message::Page(pages::Message::Screensaver(
                    pages::ScreensaverMessage::SelectLogoComplete(result),
                ))
            }),
            pages::ScreensaverMessage::SelectLogoComplete(result) => {
                match &result {
                    Ok(path) => {
                        self.screensaver_config.logo_file = path.clone();
                        tracing::info!("Logo selected: {path}");
                    }
                    Err(e) => {
                        if e != "cancelled" {
                            tracing::error!("Logo selection failed: {e}");
                        }
                    }
                }
                Task::none()
            }
            pages::ScreensaverMessage::SaveConfig => {
                self.screensaver_status_msg = None;
                Self::save_screensaver_config(
                    self.screensaver_config.clone(),
                    false,
                    self.native_idle_active,
                )
            }
            pages::ScreensaverMessage::SaveAndTest => {
                self.screensaver_status_msg = None;
                Self::save_screensaver_config(
                    self.screensaver_config.clone(),
                    true,
                    self.native_idle_active,
                )
            }
            pages::ScreensaverMessage::SaveComplete(result, launch_test) => {
                match &result {
                    Ok(()) => {
                        tracing::info!("Screensaver config saved and reloaded");
                        self.screensaver_status_msg = Some(fl!("screensaver-save-success"));
                        // Recompute idle config — iced auto-restarts subscription on change
                        self.idle_subscription_config =
                            Self::compute_idle_config(&self.screensaver_config);
                    }
                    Err(e) => {
                        tracing::error!("Failed to save screensaver config: {e}");
                        self.screensaver_status_msg = Some(fl!("screensaver-save-error"));
                    }
                }
                if launch_test && result.is_ok() {
                    // Guard against double-launch
                    if self.compositor_backup.is_some() {
                        tracing::warn!("Screensaver test already running, skipping");
                        return Task::none();
                    }

                    // Disable compositor interference via cosmic-config API
                    let backup = match compositor::disable_interference() {
                        Ok(b) => b,
                        Err(e) => {
                            tracing::warn!("Compositor disable failed: {e}");
                            None
                        }
                    };
                    self.compositor_backup.clone_from(&backup);

                    let launcher = ScreensaverConfig::fullscreen_launcher_path();
                    if !launcher.exists() {
                        tracing::warn!("launch-fullscreen.sh not found at {}", launcher.display());
                        // Restore immediately since we won't launch
                        if let Some(ref b) = self.compositor_backup.take()
                            && let Err(e) = compositor::restore_settings(b)
                        {
                            tracing::warn!("Compositor restore failed: {e}");
                        }
                        return Task::none();
                    }

                    // Spawn the screensaver and wait for it to exit
                    match std::process::Command::new(&launcher)
                        .arg("launch")
                        .arg("force")
                        .arg("--skip-compositor")
                        .spawn()
                    {
                        Ok(child) => {
                            return cosmic::task::future(async move {
                                let result = tokio::task::spawn_blocking(move || {
                                    let mut child = child;
                                    match child.wait() {
                                        Ok(status) => {
                                            if status.success() {
                                                Ok(())
                                            } else {
                                                Err(format!("Screensaver exited with: {status}"))
                                            }
                                        }
                                        Err(e) => Err(format!("Wait failed: {e}")),
                                    }
                                })
                                .await
                                .map_err(|e| format!("Spawn blocking failed: {e}"))
                                .and_then(|r| r);

                                // Restore compositor settings in the async task
                                if let Some(ref b) = backup
                                    && let Err(e) = compositor::restore_settings(b)
                                {
                                    tracing::warn!("Compositor restore failed: {e}");
                                }

                                Message::Page(pages::Message::Screensaver(
                                    pages::ScreensaverMessage::ScreensaverTestExited(result),
                                ))
                            });
                        }
                        Err(e) => {
                            tracing::error!("Failed to launch screensaver test: {e}");
                            if let Some(ref b) = self.compositor_backup.take()
                                && let Err(e) = compositor::restore_settings(b)
                            {
                                tracing::warn!("Compositor restore failed: {e}");
                            }
                        }
                    }
                }
                Task::none()
            }
            pages::ScreensaverMessage::ScreensaverTestExited(result) => {
                self.compositor_backup = None;
                match &result {
                    Ok(()) => tracing::info!("Screensaver test completed"),
                    Err(e) => tracing::warn!("Screensaver test exited: {e}"),
                }
                Task::none()
            }
        }
    }

    /// Handle Wayland idle events
    #[allow(clippy::cognitive_complexity)]
    fn handle_idle_event(&mut self, event: wayland_idle::IdleEvent) -> Task<Message> {
        match event {
            wayland_idle::IdleEvent::Connected => {
                tracing::info!("Native idle detection connected — stopping swayidle");
                self.native_idle_active = true;
                cosmic::task::future(async {
                    if let Err(e) =
                        crate::systemd::stop_user_unit("cosmic-screensaver-idle.service").await
                    {
                        tracing::warn!("Failed to stop swayidle (may not be running): {e}");
                    }
                    // Return a no-op message — we already updated state synchronously
                    Message::Page(pages::Message::Screensaver(
                        pages::ScreensaverMessage::SaveComplete(Ok(()), false),
                    ))
                })
            }
            wayland_idle::IdleEvent::ScreensaverIdle => {
                // Respect caffeine mode
                if self.caffeine_active {
                    tracing::debug!("Screensaver idle ignored — caffeine mode active");
                    return Task::none();
                }

                tracing::info!("Screensaver idle — launching screensaver");
                let launcher = ScreensaverConfig::fullscreen_launcher_path();
                if !launcher.exists() {
                    tracing::warn!("launch-fullscreen.sh not found at {}", launcher.display());
                    return Task::none();
                }

                match std::process::Command::new(&launcher)
                    .arg("launch")
                    .arg("force")
                    .arg("--skip-compositor")
                    .spawn()
                {
                    Ok(child) => {
                        self.idle_screensaver_child = Some(child.id());
                        tracing::info!("Screensaver launched (pid {})", child.id());
                    }
                    Err(e) => {
                        tracing::error!("Failed to launch screensaver: {e}");
                    }
                }
                Task::none()
            }
            wayland_idle::IdleEvent::ScreensaverResumed => {
                tracing::info!("User activity resumed — killing screensaver");
                self.kill_idle_screensaver();
                Task::none()
            }
            wayland_idle::IdleEvent::LockIdle => {
                tracing::info!("Lock idle — locking screen");
                self.lock_screen()
            }
            wayland_idle::IdleEvent::Error(e) => {
                tracing::warn!("Idle subscription error: {e} — falling back to swayidle");
                self.native_idle_active = false;
                cosmic::task::future(async {
                    if let Err(e) =
                        crate::systemd::restart_user_unit("cosmic-screensaver-idle.service").await
                    {
                        tracing::warn!("Failed to restart swayidle: {e}");
                    }
                    Message::Page(pages::Message::Screensaver(
                        pages::ScreensaverMessage::SaveComplete(Ok(()), false),
                    ))
                })
            }
        }
    }

    /// Handle logind sleep events
    fn handle_sleep_event(&mut self, event: sleep_lock::SleepEvent) -> Task<Message> {
        match event {
            sleep_lock::SleepEvent::PrepareForSleep => {
                tracing::info!("System going to sleep — locking screen");
                self.lock_screen()
            }
        }
    }

    /// Kill the screensaver child process if running
    fn kill_idle_screensaver(&mut self) {
        if self.idle_screensaver_child.take().is_some() {
            let launcher = ScreensaverConfig::fullscreen_launcher_path();
            if let Err(e) = std::process::Command::new(&launcher).arg("kill").status() {
                tracing::warn!("Failed to kill screensaver via launcher: {e}");
            }
        }
    }

    /// Lock the screen via logind D-Bus
    fn lock_screen(&self) -> Task<Message> {
        cosmic::task::future(async {
            if let Err(e) = crate::systemd::lock_session().await {
                tracing::error!("Failed to lock screen: {e}");
            }
            Message::None
        })
    }

    /// Synchronously restart swayidle — used during app exit when tokio may be shutting down
    fn restart_swayidle_sync() {
        match std::process::Command::new("systemctl")
            .args(["--user", "restart", "cosmic-screensaver-idle.service"])
            .status()
        {
            Ok(status) => {
                if status.success() {
                    tracing::info!("Swayidle service restarted on app exit");
                } else {
                    tracing::warn!("Swayidle restart exited with: {status}");
                }
            }
            Err(e) => tracing::warn!("Failed to restart swayidle on exit: {e}"),
        }
    }

    /// Compute idle subscription config from screensaver settings
    const fn compute_idle_config(
        config: &ScreensaverConfig,
    ) -> wayland_idle::IdleSubscriptionConfig {
        let screensaver_timeout_ms = if config.enabled {
            config.idle_timeout.saturating_mul(1000)
        } else {
            0
        };
        let lock_timeout_ms = if config.enabled && config.lock_timeout > 0 {
            (config.idle_timeout + config.lock_timeout).saturating_mul(1000)
        } else {
            0
        };
        wayland_idle::IdleSubscriptionConfig {
            screensaver_timeout_ms,
            lock_timeout_ms,
            enabled: config.enabled,
        }
    }

    /// Save screensaver config and reload service, optionally launching test after
    fn save_screensaver_config(
        config: ScreensaverConfig,
        launch_test: bool,
        native_idle_active: bool,
    ) -> Task<Message> {
        let dpms_timeout = config.dpms_timeout;
        cosmic::task::future(async move {
            let result: Result<(), String> = async {
                // Save config + generate swayidle conf (blocking I/O)
                let config_clone = config.clone();
                tokio::task::spawn_blocking(move || {
                    config_clone.save().map_err(|e| e.to_string())?;
                    config_clone
                        .generate_swayidle_config()
                        .map_err(|e| e.to_string())?;
                    crate::cosmic_idle::write_screen_off_time(dpms_timeout);
                    Ok(())
                })
                .await
                .map_err(|e| e.to_string())
                .and_then(|r| r)?;

                // Only restart swayidle when native idle is not active
                if !native_idle_active
                    && let Err(e) =
                        crate::systemd::restart_user_unit("cosmic-screensaver-idle.service").await
                {
                    tracing::warn!("Service restart failed (swayidle may not be running): {e}");
                }

                Ok(())
            }
            .await;

            Message::Page(pages::Message::Screensaver(
                pages::ScreensaverMessage::SaveComplete(result, launch_test),
            ))
        })
    }

    /// Logo file picker dialog
    async fn run_logo_select() -> Result<String, String> {
        use cosmic::dialog::file_chooser;

        let dialog = file_chooser::open::Dialog::new()
            .title(fl!("screensaver-logo"))
            .filter(file_chooser::FileFilter::new(&fl!("filter-text-files")).glob("*.txt"));

        let response = match dialog.open_file().await {
            Ok(r) => r,
            Err(file_chooser::Error::Cancelled) => return Err("cancelled".to_string()),
            Err(e) => return Err(format!("Dialog error: {e}")),
        };

        let url = response.url();
        let path = url
            .to_file_path()
            .map_err(|_| "Invalid file path".to_string())?;

        Ok(path.to_string_lossy().to_string())
    }

    /// Build a timeout slider row with label, slider, and value display
    fn view_timeout_slider<'a, F>(
        label_text: String,
        value_seconds: u32,
        ticks: &[u32; 7],
        on_change: F,
    ) -> Element<'a, Message>
    where
        F: Fn(u32) -> Message + 'a,
    {
        use cosmic::iced::Length;
        let spacing = cosmic::theme::spacing();
        let minutes = value_seconds / 60;

        // Find nearest tick index for current value
        let index = ticks
            .iter()
            .enumerate()
            .min_by_key(|(_, v)| (**v as i32 - minutes as i32).unsigned_abs())
            .map(|(i, _)| i as u32)
            .unwrap_or(0);

        // Value display
        let value_label = if minutes == 0 {
            fl!("screensaver-timeout-disabled")
        } else if minutes >= 60 {
            fl!("timeout-hours", hours = (minutes / 60).to_string())
        } else {
            fl!("timeout-minutes", minutes = minutes.to_string())
        };

        let ticks_owned = *ticks;
        let slider = widget::slider(0u32..=6u32, index, move |idx| {
            let min = ticks_owned.get(idx as usize).copied().unwrap_or(0);
            on_change(min * 60)
        });

        widget::row()
            .spacing(spacing.space_s)
            .align_y(cosmic::iced::Alignment::Center)
            .push(widget::text::body(label_text).width(Length::Fixed(120.0)))
            .push(slider)
            .push(widget::text::body(value_label).width(Length::Fixed(80.0)))
            .into()
    }

    /// Fade effect option values (empty string = None)
    fn fade_effect_values() -> Vec<String> {
        vec![
            String::new(),
            "fade".to_string(),
            "slide".to_string(),
            "matrix".to_string(),
        ]
    }

    /// Collect the visible wallpaper paths for the current page and spawn
    /// background thumbnail generation for any that are not yet cached.
    fn spawn_thumbnail_generation(&self) -> Task<Message> {
        const PER_PAGE: usize = 12;

        let paths: Vec<String> = if let Some(theme) = self
            .wallpaper_selected_collection
            .as_ref()
            .and_then(|c| self.wallpaper_config.available_themes.get(c))
        {
            let total = theme.wallpapers.len();
            let total_pages = total.div_ceil(PER_PAGE);
            let page = self.wallpaper_grid_page.min(total_pages.saturating_sub(1));
            let start = page * PER_PAGE;
            let end = (start + PER_PAGE).min(total);
            theme.wallpapers[start..end]
                .iter()
                .map(|filename| theme.path.join(filename).to_string_lossy().to_string())
                .collect()
        } else {
            Vec::new()
        };

        // Filter to only those not yet cached
        let missing: Vec<String> = paths
            .into_iter()
            .filter(|p| self.thumbnail_cache.get_cached(p).is_none())
            .collect();

        if missing.is_empty() {
            return Task::none();
        }

        let cache = ThumbnailCache {
            cache_dir: self.thumbnail_cache.cache_dir.clone(),
        };

        cosmic::task::future(async move {
            tokio::task::spawn_blocking(move || {
                cache.generate_batch(&missing);
            })
            .await
            .ok();

            Message::Page(pages::Message::Wallpapers(
                pages::WallpapersMessage::ThumbnailsReady,
            ))
        })
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
                file_chooser::FileFilter::new(&fl!("filter-images"))
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
            .filter(file_chooser::FileFilter::new(&fl!("filter-ron-theme")).glob("*.ron"));

        let response = match dialog.save_file().await {
            Ok(r) => r,
            Err(file_chooser::Error::Cancelled) => return Err("cancelled".to_string()),
            Err(e) => return Err(format!("Dialog error: {e}")),
        };

        let url = response
            .url()
            .ok_or_else(|| "No file URL returned".to_string())?;
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
            .filter(file_chooser::FileFilter::new(&fl!("filter-ron-theme")).glob("*.ron"));

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

        column =
            column
                // Theme presets
                .push(widget::settings::section().title(fl!("theme-presets")).add(
                    widget::settings::item(fl!("available-themes"), self.view_theme_list()),
                ))
                // Mode selection
                .push(widget::settings::section().title(fl!("theme-mode")).add(
                    widget::settings::item(
                        fl!("dark-mode"),
                        widget::toggler(cfg.is_dark).on_toggle(|enabled| {
                            Message::Page(pages::Message::Themes(
                                pages::ThemesMessage::SetDarkMode(enabled),
                            ))
                        }),
                    ),
                ))
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
                                .push(widget::tooltip(
                                    widget::button::standard(fl!("theme-export")).on_press(
                                        Message::Page(pages::Message::Themes(
                                            pages::ThemesMessage::Export,
                                        )),
                                    ),
                                    widget::text::body(fl!("tooltip-export")),
                                    widget::tooltip::Position::Top,
                                )),
                        )
                        .add(
                            widget::column()
                                .spacing(spacing.space_xs)
                                .push(widget::text::body(fl!("theme-import-description")))
                                .push(widget::tooltip(
                                    widget::button::standard(fl!("theme-import")).on_press(
                                        Message::Page(pages::Message::Themes(
                                            pages::ThemesMessage::Import,
                                        )),
                                    ),
                                    widget::text::body(fl!("tooltip-import")),
                                    widget::tooltip::Position::Top,
                                )),
                        ),
                )
                // Tool Sync
                .push(self.view_tool_sync_section());

        widget::scrollable(column).into()
    }

    /// Create tool sync settings section
    fn view_tool_sync_section(&self) -> Element<'_, Message> {
        let spacing = cosmic::theme::spacing();

        let mut section = widget::settings::section()
            .title(fl!("tool-sync"))
            .add(widget::settings::item(
                fl!("tool-sync-auto"),
                widget::toggler(self.tool_sync_config.auto_sync).on_toggle(|enabled| {
                    Message::Page(pages::Message::Themes(pages::ThemesMessage::SetAutoSync(
                        enabled,
                    )))
                }),
            ))
            .add(widget::settings::item(
                fl!("tool-sync-ghostty"),
                widget::toggler(self.tool_sync_config.ghostty_enabled).on_toggle(|enabled| {
                    Message::Page(pages::Message::Themes(
                        pages::ThemesMessage::SetGhosttySync(enabled),
                    ))
                }),
            ))
            .add(widget::settings::item(
                fl!("tool-sync-btop"),
                widget::toggler(self.tool_sync_config.btop_enabled).on_toggle(|enabled| {
                    Message::Page(pages::Message::Themes(pages::ThemesMessage::SetBtopSync(
                        enabled,
                    )))
                }),
            ))
            .add(widget::settings::item(
                fl!("tool-sync-nvim"),
                widget::toggler(self.tool_sync_config.nvim_enabled).on_toggle(|enabled| {
                    Message::Page(pages::Message::Themes(pages::ThemesMessage::SetNvimSync(
                        enabled,
                    )))
                }),
            ))
            .add(widget::settings::item(
                fl!("tool-sync-zellij"),
                widget::toggler(self.tool_sync_config.zellij_enabled).on_toggle(|enabled| {
                    Message::Page(pages::Message::Themes(pages::ThemesMessage::SetZellijSync(
                        enabled,
                    )))
                }),
            ))
            .add(widget::settings::item(
                fl!("tool-sync-fzf"),
                widget::toggler(self.tool_sync_config.fzf_enabled).on_toggle(|enabled| {
                    Message::Page(pages::Message::Themes(pages::ThemesMessage::SetFzfSync(
                        enabled,
                    )))
                }),
            ))
            .add(widget::settings::item(
                fl!("tool-sync-fzf-shell"),
                widget::toggler(self.tool_sync_config.fzf_shell_integration).on_toggle(|enabled| {
                    Message::Page(pages::Message::Themes(
                        pages::ThemesMessage::SetFzfShellIntegration(enabled),
                    ))
                }),
            ))
            .add(widget::settings::item(
                fl!("tool-sync-lazygit"),
                widget::toggler(self.tool_sync_config.lazygit_enabled).on_toggle(|enabled| {
                    Message::Page(pages::Message::Themes(
                        pages::ThemesMessage::SetLazygitSync(enabled),
                    ))
                }),
            ))
            .add(widget::settings::item(
                fl!("tool-sync-hooks"),
                widget::toggler(self.tool_sync_config.hooks_enabled).on_toggle(|enabled| {
                    Message::Page(pages::Message::Themes(
                        pages::ThemesMessage::SetHooksEnabled(enabled),
                    ))
                }),
            ));

        // Sync button + status row
        let mut sync_row = widget::row().spacing(spacing.space_m).push(widget::tooltip(
            widget::button::suggested(fl!("tool-sync-now")).on_press(Message::Page(
                pages::Message::Themes(pages::ThemesMessage::SyncTools),
            )),
            widget::text::body(fl!("tool-sync-description")),
            widget::tooltip::Position::Top,
        ));

        if let Some(ref status) = self.tool_sync_status {
            sync_row = sync_row.push(widget::text::body(status));
        }

        section = section.add(sync_row);

        section.into()
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
        let previewing_id = self.theme_preview_backup.as_ref().map(|b| b.previewing_id);

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
                        widget::container(widget::Space::new(Length::Fill, Length::Fixed(6.0)))
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
                        .class(cosmic::theme::Container::custom(
                            move |_| widget::container::Style {
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
                            },
                        )),
                    )
                    // Text line 2 (shorter)
                    .push(
                        widget::container(widget::Space::new(
                            Length::Fixed(56.0),
                            Length::Fixed(4.0),
                        ))
                        .class(cosmic::theme::Container::custom(
                            move |_| widget::container::Style {
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
                            },
                        )),
                    )
                    // Text line 3 (medium)
                    .push(
                        widget::container(widget::Space::new(
                            Length::Fixed(66.0),
                            Length::Fixed(4.0),
                        ))
                        .class(cosmic::theme::Container::custom(
                            move |_| widget::container::Style {
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
                            },
                        )),
                    )
                    // Small accent button mockup at bottom
                    .push(
                        widget::container(widget::Space::new(
                            Length::Fixed(36.0),
                            Length::Fixed(8.0),
                        ))
                        .class(cosmic::theme::Container::custom(
                            move |_| widget::container::Style {
                                background: Some(cosmic::iced::Background::Color(
                                    cosmic::iced::Color { a: 0.8, ..accent },
                                )),
                                border: cosmic::iced::Border {
                                    radius: 3.0.into(),
                                    ..Default::default()
                                },
                                ..Default::default()
                            },
                        )),
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
                .push(widget::tooltip(
                    widget::button::standard(fl!("theme-try")).on_press(Message::Page(
                        pages::Message::Themes(pages::ThemesMessage::PreviewTheme(theme_id)),
                    )),
                    widget::text::body(fl!("tooltip-try")),
                    widget::tooltip::Position::Top,
                ));

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
                background: Some(cosmic::iced::Background::Color(
                    cosmic::iced::Color::from_rgba(accent.red, accent.green, accent.blue, 0.15),
                )),
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
        let apply_with_tooltip = widget::tooltip(
            apply_button,
            widget::text::body(fl!("tooltip-apply")),
            widget::tooltip::Position::Top,
        );

        let import_button =
            widget::button::standard(fl!("wallpaper-add-file")).on_press(Message::Page(
                pages::Message::Wallpapers(pages::WallpapersMessage::ImportFromFile),
            ));
        let import_with_tooltip = widget::tooltip(
            import_button,
            widget::text::body(fl!("tooltip-import")),
            widget::tooltip::Position::Top,
        );

        buttons_row = buttons_row
            .push(apply_with_tooltip)
            .push(import_with_tooltip);

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
        let wallpapers: Vec<(String, String)> = if let Some(theme) = self
            .wallpaper_selected_collection
            .as_ref()
            .and_then(|c| cfg.available_themes.get(c))
        {
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
            nav_row = nav_row.push(widget::tooltip(
                widget::button::standard("<").on_press(Message::Page(pages::Message::Wallpapers(
                    pages::WallpapersMessage::GridPrevPage,
                ))),
                widget::text::body(fl!("tooltip-prev-page")),
                widget::tooltip::Position::Top,
            ));
        } else {
            nav_row = nav_row.push(widget::button::standard("<"));
        }

        nav_row = nav_row.push(widget::text::body(format!(
            "{} / {}",
            page + 1,
            total_pages,
        )));

        if page + 1 < total_pages {
            nav_row = nav_row.push(widget::tooltip(
                widget::button::standard(">").on_press(Message::Page(pages::Message::Wallpapers(
                    pages::WallpapersMessage::GridNextPage,
                ))),
                widget::text::body(fl!("tooltip-next-page")),
                widget::tooltip::Position::Top,
            ));
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
    fn view_wallpaper_card<'a>(&'a self, full_path: &str, filename: &str) -> Element<'a, Message> {
        use cosmic::iced::Length;
        use cosmic::widget::image::Handle;

        let spacing = cosmic::theme::spacing();
        let is_current = self.wallpaper_config.current_source == full_path;
        let is_selected = self.wallpaper_selected_path.as_deref() == Some(full_path);

        let path_owned = full_path.to_string();

        let image_button = if let Some(thumb_path) = self.thumbnail_cache.get_cached(full_path) {
            widget::button::image(Handle::from_path(thumb_path))
                .width(Length::Fixed(160.0))
                .height(Length::Fixed(100.0))
                .selected(is_current || is_selected)
                .on_press(Message::Page(pages::Message::Wallpapers(
                    pages::WallpapersMessage::SelectWallpaper(path_owned),
                )))
        } else {
            // Placeholder — thumbnail will be generated in background
            widget::button::image(Handle::from_path(PathBuf::from(
                "/usr/share/icons/hicolor/scalable/apps/image-missing.svg",
            )))
            .width(Length::Fixed(160.0))
            .height(Length::Fixed(100.0))
            .selected(is_current || is_selected)
            .on_press(Message::Page(pages::Message::Wallpapers(
                pages::WallpapersMessage::SelectWallpaper(path_owned),
            )))
        };

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

        let scaling_dropdown = widget::dropdown(scaling_options, scaling_selected, |index| {
            Message::Page(pages::Message::Wallpapers(
                pages::WallpapersMessage::SetScalingMode(index),
            ))
        });

        // Save button
        let save_button = widget::tooltip(
            widget::button::suggested(fl!("save")).on_press(Message::Page(
                pages::Message::Wallpapers(pages::WallpapersMessage::SaveSettings),
            )),
            widget::text::body(fl!("tooltip-save")),
            widget::tooltip::Position::Top,
        );

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
            .add(widget::row().spacing(spacing.space_s).push(save_button))
            .into()
    }

    /// Build the logo section: dropdown selector + from-file button on separate lines
    fn view_screensaver_logo_section(&self) -> Element<'_, Message> {
        // Logo dropdown: list of available logos
        let logo_options: Vec<String> = self
            .available_logos
            .iter()
            .map(|(name, _)| name.clone())
            .collect();

        let logo_selected = self
            .available_logos
            .iter()
            .position(|(_, path)| path.to_string_lossy() == self.screensaver_config.logo_file);

        let logos_for_closure = self.available_logos.clone();
        let logo_dropdown = widget::dropdown(logo_options, logo_selected, move |index| {
            let path = logos_for_closure
                .get(index)
                .map(|(_, p)| p.to_string_lossy().to_string())
                .unwrap_or_default();
            Message::Page(pages::Message::Screensaver(
                pages::ScreensaverMessage::SelectLogo(path),
            ))
        });

        // From File button
        let from_file_button =
            widget::button::standard(fl!("screensaver-select-logo")).on_press(Message::Page(
                pages::Message::Screensaver(pages::ScreensaverMessage::SelectLogoDialog),
            ));

        widget::settings::section()
            .title(fl!("screensaver-logo"))
            .add(widget::settings::item(
                fl!("screensaver-logo-available"),
                logo_dropdown,
            ))
            .add(widget::settings::item(
                fl!("screensaver-logo-load"),
                from_file_button,
            ))
            .into()
    }

    /// View for the Screensaver page
    #[allow(clippy::cognitive_complexity)]
    fn view_screensaver_page(&self) -> Element<'_, Message> {
        let spacing = cosmic::theme::spacing();
        let cfg = &self.screensaver_config;

        // --- Timeout sliders with tick marks ---
        // All three sliders use the same tick values (index 0..=6)
        let ticks: [u32; 7] = [0, 5, 10, 15, 30, 45, 60];

        let idle_slider = Self::view_timeout_slider(
            fl!("screensaver-idle-timeout"),
            cfg.idle_timeout,
            &ticks,
            |seconds| {
                Message::Page(pages::Message::Screensaver(
                    pages::ScreensaverMessage::SetIdleTimeout(seconds),
                ))
            },
        );

        let lock_slider = Self::view_timeout_slider(
            fl!("screensaver-lock-timeout"),
            cfg.lock_timeout,
            &ticks,
            |seconds| {
                Message::Page(pages::Message::Screensaver(
                    pages::ScreensaverMessage::SetLockTimeout(seconds),
                ))
            },
        );

        let dpms_slider = Self::view_timeout_slider(
            fl!("screensaver-dpms-timeout"),
            cfg.dpms_timeout,
            &ticks,
            |seconds| {
                Message::Page(pages::Message::Screensaver(
                    pages::ScreensaverMessage::SetDpmsTimeout(seconds),
                ))
            },
        );

        // --- Frame rate dropdown ---
        let fps_options: Vec<String> = vec![
            fl!("screensaver-fps-30"),
            fl!("screensaver-fps-60"),
            fl!("screensaver-fps-120"),
        ];
        let fps_values: [u32; 3] = [30, 60, 120];
        let fps_selected = fps_values.iter().position(|&v| v == cfg.frame_rate);
        let fps_dropdown = widget::dropdown(fps_options, fps_selected, |index| {
            Message::Page(pages::Message::Screensaver(
                pages::ScreensaverMessage::SetFrameRate(index),
            ))
        });

        // --- Fade effect dropdowns ---
        let fade_labels: Vec<String> = vec![
            fl!("screensaver-fade-none"),
            "fade".to_string(),
            "slide".to_string(),
            "matrix".to_string(),
        ];
        let fade_values = Self::fade_effect_values();
        let fade_in_selected = fade_values.iter().position(|v| v == &cfg.fade_in_effect);
        let fade_out_selected = fade_values.iter().position(|v| v == &cfg.fade_out_effect);

        let fade_in_labels = fade_labels.clone();
        let fade_in_dropdown = widget::dropdown(fade_in_labels, fade_in_selected, |index| {
            Message::Page(pages::Message::Screensaver(
                pages::ScreensaverMessage::SetFadeInEffect(index),
            ))
        });
        let fade_out_dropdown = widget::dropdown(fade_labels, fade_out_selected, |index| {
            Message::Page(pages::Message::Screensaver(
                pages::ScreensaverMessage::SetFadeOutEffect(index),
            ))
        });

        // --- Exclude/Include effects presets ---
        let exclude_preset_labels: Vec<String> = vec![
            fl!("screensaver-effects-none"),
            fl!("screensaver-effects-preset-default"),
            fl!("screensaver-effects-preset-heavy"),
        ];
        let exclude_preset_values: Vec<String> = vec![
            String::new(),
            "dev_worm".to_string(),
            "blackhole,burn,fireworks,orbittingvolley,overflow".to_string(),
        ];
        let exclude_selected = exclude_preset_values
            .iter()
            .position(|v| v == &cfg.exclude_effects);
        let exclude_dropdown =
            widget::dropdown(exclude_preset_labels, exclude_selected, move |index| {
                let value = exclude_preset_values
                    .get(index)
                    .cloned()
                    .unwrap_or_default();
                Message::Page(pages::Message::Screensaver(
                    pages::ScreensaverMessage::SetExcludeEffects(value),
                ))
            });

        let include_preset_labels: Vec<String> = vec![
            fl!("screensaver-effects-all-except"),
            fl!("screensaver-effects-preset-simple"),
            fl!("screensaver-effects-preset-colorful"),
        ];
        let include_preset_values: Vec<String> = vec![
            String::new(),
            "beams,colorshift,decrypt,expand,middleout,pour,print,slide,waves,wipe".to_string(),
            "beams,binarypath,colorshift,fireworks,rain,rings,synthgrid".to_string(),
        ];
        let include_selected = include_preset_values
            .iter()
            .position(|v| v == &cfg.include_effects);
        let include_dropdown =
            widget::dropdown(include_preset_labels, include_selected, move |index| {
                let value = include_preset_values
                    .get(index)
                    .cloned()
                    .unwrap_or_default();
                Message::Page(pages::Message::Screensaver(
                    pages::ScreensaverMessage::SetIncludeEffects(value),
                ))
            });

        // --- Clock dropdowns ---
        let clock_fmt_options: Vec<String> = vec![
            fl!("screensaver-clock-24h"),
            fl!("screensaver-clock-24h-sec"),
            fl!("screensaver-clock-12h"),
            fl!("screensaver-clock-12h-sec"),
        ];
        let clock_fmt_values = ["%H:%M", "%H:%M:%S", "%I:%M %p", "%I:%M:%S %p"];
        let clock_fmt_selected = clock_fmt_values.iter().position(|&v| v == cfg.clock_format);
        let clock_fmt_dropdown = widget::dropdown(clock_fmt_options, clock_fmt_selected, |index| {
            Message::Page(pages::Message::Screensaver(
                pages::ScreensaverMessage::SetClockFormat(index),
            ))
        });

        let clock_dur_options: Vec<String> = vec![
            fl!("screensaver-clock-3sec"),
            fl!("screensaver-clock-5sec"),
            fl!("screensaver-clock-10sec"),
            fl!("screensaver-clock-15sec"),
        ];
        let clock_dur_values: [u32; 4] = [3, 5, 10, 15];
        let clock_dur_selected = clock_dur_values
            .iter()
            .position(|&v| v == cfg.clock_duration);
        let clock_dur_dropdown = widget::dropdown(clock_dur_options, clock_dur_selected, |index| {
            Message::Page(pages::Message::Screensaver(
                pages::ScreensaverMessage::SetClockDuration(index),
            ))
        });

        // --- Terminal dropdown ---
        let terminal_options: Vec<String> = vec![
            fl!("screensaver-terminal-ghostty"),
            fl!("screensaver-terminal-cosmic-term"),
        ];
        let terminal_values = ["ghostty", "cosmic-term"];
        let terminal_selected = terminal_values.iter().position(|&v| v == cfg.terminal);
        let terminal_dropdown = widget::dropdown(terminal_options, terminal_selected, |index| {
            Message::Page(pages::Message::Screensaver(
                pages::ScreensaverMessage::SetTerminal(index),
            ))
        });

        // --- Build page ---
        let column = widget::column()
            .spacing(spacing.space_m)
            .padding(spacing.space_m)
            .push(widget::text::title2(fl!("screensaver")))
            .push(widget::text::body(fl!("screensaver-description")))
            // Status section
            .push(
                widget::settings::section()
                    .title(fl!("screensaver-status"))
                    .add(widget::settings::item(
                        fl!("screensaver-enabled"),
                        widget::toggler(cfg.enabled).on_toggle(|enabled| {
                            Message::Page(pages::Message::Screensaver(
                                pages::ScreensaverMessage::SetEnabled(enabled),
                            ))
                        }),
                    ))
                    .add(widget::settings::item(
                        fl!("screensaver-terminal"),
                        terminal_dropdown,
                    )),
            )
            // Logo section
            .push(self.view_screensaver_logo_section())
            // Timeouts section
            .push(
                widget::settings::section()
                    .title(fl!("screensaver-timeouts"))
                    .add(idle_slider)
                    .add(lock_slider)
                    .add(dpms_slider),
            )
            // Effects section
            .push(
                widget::settings::section()
                    .title(fl!("screensaver-effects"))
                    .add(widget::settings::item(
                        fl!("screensaver-frame-rate"),
                        fps_dropdown,
                    ))
                    .add(widget::settings::item(
                        fl!("screensaver-effects-exclude"),
                        exclude_dropdown,
                    ))
                    .add(widget::settings::item(
                        fl!("screensaver-effects-include"),
                        include_dropdown,
                    ))
                    .add(widget::settings::item(
                        fl!("screensaver-fade-in"),
                        fade_in_dropdown,
                    ))
                    .add(widget::settings::item(
                        fl!("screensaver-fade-out"),
                        fade_out_dropdown,
                    )),
            )
            // Clock section
            .push(
                widget::settings::section()
                    .title(fl!("screensaver-clock"))
                    .add(widget::settings::item(
                        fl!("screensaver-clock-enabled"),
                        widget::toggler(cfg.show_clock).on_toggle(|enabled| {
                            Message::Page(pages::Message::Screensaver(
                                pages::ScreensaverMessage::SetShowClock(enabled),
                            ))
                        }),
                    ))
                    .add(widget::settings::item(
                        fl!("screensaver-clock-duration"),
                        clock_dur_dropdown,
                    ))
                    .add(widget::settings::item(
                        fl!("screensaver-clock-format"),
                        clock_fmt_dropdown,
                    )),
            )
            // Cursor & Dismiss section
            .push(
                widget::settings::section()
                    .title(fl!("screensaver-cursor-section"))
                    .add(widget::settings::item(
                        fl!("screensaver-cursor-hide"),
                        widget::toggler(cfg.cursor_hide).on_toggle(|enabled| {
                            Message::Page(pages::Message::Screensaver(
                                pages::ScreensaverMessage::SetCursorHide(enabled),
                            ))
                        }),
                    ))
                    .add(widget::settings::item(
                        fl!("screensaver-hide-mouse"),
                        widget::toggler(cfg.hide_mouse).on_toggle(|enabled| {
                            Message::Page(pages::Message::Screensaver(
                                pages::ScreensaverMessage::SetHideMouse(enabled),
                            ))
                        }),
                    ))
                    .add(widget::settings::item(
                        fl!("screensaver-dismiss-on-key"),
                        widget::toggler(cfg.dismiss_on_key).on_toggle(|enabled| {
                            Message::Page(pages::Message::Screensaver(
                                pages::ScreensaverMessage::SetDismissOnKey(enabled),
                            ))
                        }),
                    )),
            )
            // Action buttons + status
            .push({
                let mut action_col = widget::column().spacing(spacing.space_s);

                // Status message (if any)
                if let Some(ref msg) = self.screensaver_status_msg {
                    action_col = action_col.push(widget::text::body(msg.clone()));
                }

                let save_btn = widget::tooltip(
                    widget::button::suggested(fl!("screensaver-save")).on_press(Message::Page(
                        pages::Message::Screensaver(pages::ScreensaverMessage::SaveConfig),
                    )),
                    widget::text::body(fl!("tooltip-save")),
                    widget::tooltip::Position::Top,
                );
                let test_btn = widget::tooltip(
                    widget::button::standard(fl!("screensaver-save-test")).on_press(Message::Page(
                        pages::Message::Screensaver(pages::ScreensaverMessage::SaveAndTest),
                    )),
                    widget::text::body(fl!("tooltip-save-test")),
                    widget::tooltip::Position::Top,
                );
                action_col = action_col.push(
                    widget::row()
                        .spacing(spacing.space_s)
                        .push(save_btn)
                        .push(test_btn),
                );

                action_col
            });

        widget::scrollable(column).into()
    }
}

impl Drop for App {
    fn drop(&mut self) {
        if self.native_idle_active {
            // Safety net: restart swayidle when the app is dropped
            Self::restart_swayidle_sync();
        }
    }
}
