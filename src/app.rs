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
use crate::wayland_idle;
use std::path::PathBuf;

/// Application state
#[allow(clippy::struct_excessive_bools)]
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
    /// Backup of theme state during preview (None when not previewing)
    theme_preview_backup: Option<ThemePreviewState>,
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
    /// Whether session lock on idle is enabled in config
    session_lock_enabled: bool,
    /// Abort handle for the lock delay timer (cancelled on user resume)
    lock_timer_handle: Option<cosmic::iced::task::Handle>,
}

/// Application messages
#[derive(Debug, Clone)]
pub enum Message {
    /// Navigation item selected (constructed by libcosmic framework)
    #[allow(dead_code)]
    NavSelect(nav_bar::Id),
    /// Page-specific message
    Page(pages::Message),
    /// Configuration changed (constructed by cosmic-config subscription)
    #[allow(dead_code)]
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
    /// Lock screen timer elapsed
    LockScreen,
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
            return self.update(Message::Page(pages::Message::Visuals(
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
            return self.update(Message::Page(pages::Message::Visuals(
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

        // Build navigation model
        let mut nav_model = nav_bar::Model::default();

        // Add navigation items
        nav_model
            .insert()
            .text(fl!("visuals"))
            .icon(widget::icon::from_name(
                "preferences-desktop-theme-symbolic",
            ))
            .data(PageId::Visuals);

        nav_model
            .insert()
            .text(fl!("tool-sync"))
            .icon(widget::icon::from_name("preferences-other-symbolic"))
            .data(PageId::ToolSync);

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
            PageId::Visuals => 0,
            PageId::ToolSync => 1,
            PageId::Screensaver => 2,
        };
        nav_model.activate_position(position);

        // Scan available logos
        let available_logos = ScreensaverConfig::scan_logos();
        tracing::info!("Found {} available logos", available_logos.len());

        let idle_subscription_config = Self::compute_idle_config(&screensaver_config);
        let session_lock_enabled = screensaver_config.session_lock;
        let app = Self {
            core,
            config,
            nav_model,
            active_page,
            screensaver_config,
            theme_config,
            theme_preview_backup: None,
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
            session_lock_enabled,
            lock_timer_handle: None,
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
            if let Some(snapshot) = &backup.snapshot {
                if let Err(e) = crate::bundled_themes::restore_theme(snapshot) {
                    tracing::error!("Failed to restore theme on nav: {e}");
                }
            } else if let Err(e) = ThemeConfig::set_dark_mode(backup.config.is_dark) {
                tracing::error!("Failed to restore theme on nav: {e}");
            }
            self.theme_config = backup.config;
            tracing::info!("Theme preview auto-cancelled on navigation");
        }

        // Get the page ID from the navigation item
        if let Some(page_id) = self.nav_model.data::<PageId>(id).copied() {
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
        let subs = vec![
            power::power_subscription().map(Message::PowerStateUpdate),
            wayland_idle::idle_subscription(self.idle_subscription_config.clone())
                .map(Message::IdleEvent),
            sleep_lock::sleep_lock_subscription().map(Message::SleepEvent),
        ];

        cosmic::iced::Subscription::batch(subs)
    }

    fn on_app_exit(&mut self) -> Option<Self::Message> {
        // Restore theme if preview was active
        if let Some(backup) = self.theme_preview_backup.take() {
            if let Some(snapshot) = &backup.snapshot {
                if let Err(e) = crate::bundled_themes::restore_theme(snapshot) {
                    tracing::error!("Failed to restore theme on exit: {e}");
                }
            } else if let Err(e) =
                crate::theme_config::ThemeConfig::set_dark_mode(backup.config.is_dark)
            {
                tracing::error!("Failed to restore theme on exit: {e}");
            }
        }

        if self.native_idle_active {
            self.kill_idle_screensaver();
            Self::restart_swayidle_sync();
        }
        None
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
            Message::LockScreen => {
                tracing::info!("Lock timer elapsed — locking screen");
                self.lock_timer_handle = None;
                self.lock_screen()
            }
            Message::None => Task::none(),
        }
    }

    fn view(&self) -> Element<'_, Self::Message> {
        // Build page content based on active page
        match self.active_page {
            PageId::Visuals => self.view_visuals_page(),
            PageId::ToolSync => self.view_tool_sync_page(),
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
            pages::Message::Visuals(msg) => self.handle_themes_message(msg),
            pages::Message::Screensaver(msg) => self.handle_screensaver_message(msg),
        }
    }

    /// Handle theme page messages
    #[allow(clippy::cognitive_complexity, clippy::too_many_lines)]
    fn handle_themes_message(&mut self, message: pages::ThemesMessage) -> Task<Message> {
        match message {
            pages::ThemesMessage::Export => {
                let default_name = crate::theme_config::ThemeConfig::default_export_filename();
                cosmic::task::future(async move {
                    let result = Self::run_theme_export(default_name).await;
                    Message::Page(pages::Message::Visuals(
                        pages::ThemesMessage::ExportComplete(result),
                    ))
                })
            }
            pages::ThemesMessage::ExportComplete(result) => {
                match &result {
                    Ok(path) => tracing::info!("Theme exported to: {path}"),
                    Err(e) => {
                        if e == "cancelled" {
                            tracing::debug!("Theme export cancelled by user");
                        } else {
                            tracing::error!("Theme export failed: {e}");
                        }
                    }
                }
                Task::none()
            }
            pages::ThemesMessage::Import => cosmic::task::future(async move {
                let result = Self::run_theme_import().await;
                Message::Page(pages::Message::Visuals(
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
                        if e == "cancelled" {
                            tracing::debug!("Theme import cancelled by user");
                        } else {
                            tracing::error!("Theme import failed: {e}");
                        }
                    }
                }
                Task::none()
            }
            pages::ThemesMessage::PreviewTheme(theme_id) => {
                // Snapshot before first preview
                if self.theme_preview_backup.is_none() {
                    let snapshot = match crate::bundled_themes::snapshot_current_theme() {
                        Ok(s) => Some(s),
                        Err(e) => {
                            tracing::warn!("Failed to snapshot theme: {e}");
                            None
                        }
                    };
                    self.theme_preview_backup = Some(ThemePreviewState {
                        config: self.theme_config.clone(),
                        previewing_id: theme_id,
                        snapshot,
                    });
                } else if let Some(ref mut backup) = self.theme_preview_backup {
                    backup.previewing_id = theme_id;
                }

                if let crate::theme_config::ThemeId::Bundled(idx) = theme_id {
                    if let Err(e) = crate::bundled_themes::apply_bundled_theme(idx) {
                        tracing::error!("Failed to preview bundled theme: {e}");
                        self.theme_preview_backup = None;
                    } else if let Some((meta, _)) = crate::bundled_themes::all_themes().get(idx) {
                        self.theme_config.is_dark = meta.is_dark;
                        self.theme_config.name.clone_from(&meta.name);
                        tracing::info!("Previewing bundled theme: {}", meta.name);
                    }
                } else {
                    let previews = crate::theme_config::ThemePreview::built_in_themes();
                    if let Some(preview) = previews.iter().find(|p| p.id == theme_id) {
                        if let Err(e) = preview.apply() {
                            tracing::error!("Failed to preview theme: {e}");
                            self.theme_preview_backup = None;
                        } else {
                            self.theme_config.is_dark = preview.is_dark;
                            self.theme_config.name.clone_from(&preview.name);
                            tracing::info!("Previewing theme: {}", preview.name);
                        }
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
                    if let Some(snapshot) = &backup.snapshot {
                        if let Err(e) = crate::bundled_themes::restore_theme(snapshot) {
                            tracing::error!("Failed to restore theme from snapshot: {e}");
                        } else {
                            self.theme_config = backup.config;
                            tracing::info!("Theme preview cancelled, restored previous theme");
                        }
                    } else if let Err(e) = ThemeConfig::set_dark_mode(backup.config.is_dark) {
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
                    Message::Page(pages::Message::Visuals(pages::ThemesMessage::SyncComplete(
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
                    Message::Page(pages::Message::Visuals(pages::ThemesMessage::SyncComplete(
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
                    Message::Page(pages::Message::Visuals(pages::ThemesMessage::SyncComplete(
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
                    Message::Page(pages::Message::Visuals(pages::ThemesMessage::SyncComplete(
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
                    Message::Page(pages::Message::Visuals(pages::ThemesMessage::SyncComplete(
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
                    Message::Page(pages::Message::Visuals(pages::ThemesMessage::SyncComplete(
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
                    Message::Page(pages::Message::Visuals(pages::ThemesMessage::SyncComplete(
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
                    Message::Page(pages::Message::Visuals(pages::ThemesMessage::SyncComplete(
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
                    Message::Page(pages::Message::Visuals(pages::ThemesMessage::SyncComplete(
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
                            let live = crate::tool_sync::signal_running_apps(&config);

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
                            if let Some(ref hr) = r.hooks_result
                                && hr.hooks_run > 0
                            {
                                parts.push(format!(
                                    "hooks: {}/{} ok",
                                    hr.hooks_succeeded, hr.hooks_run
                                ));
                            }
                            if !live.is_empty() {
                                parts.push(format!("live: {}", live.join(", ")));
                            }
                            Ok(parts.join(", "))
                        }
                        Err(e) => Err(e),
                    };
                    Message::Page(pages::Message::Visuals(pages::ThemesMessage::SyncComplete(
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

    /// Handle screensaver page messages
    #[allow(clippy::cognitive_complexity, clippy::too_many_lines)]
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
                    self.screensaver_config.fade_in_effect.clone_from(effect);
                }
                Task::none()
            }
            pages::ScreensaverMessage::SetFadeOutEffect(index) => {
                let effects = Self::fade_effect_values();
                if let Some(effect) = effects.get(index) {
                    self.screensaver_config.fade_out_effect.clone_from(effect);
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
            pages::ScreensaverMessage::SetSessionLock(enabled) => {
                self.screensaver_config.session_lock = enabled;
                self.session_lock_enabled = enabled;
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
                        self.screensaver_config.logo_file.clone_from(path);
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

                // Schedule lock timer if session lock is enabled
                let lock_timeout = self.screensaver_config.lock_timeout;
                if self.session_lock_enabled && lock_timeout > 0 && self.lock_timer_handle.is_none()
                {
                    tracing::info!("Scheduling lock in {lock_timeout}s");
                    let (task, handle) = Task::abortable(cosmic::task::future(async move {
                        tokio::time::sleep(std::time::Duration::from_secs(u64::from(lock_timeout)))
                            .await;
                        Message::LockScreen
                    }));
                    self.lock_timer_handle = Some(handle);
                    return task;
                }
                Task::none()
            }
            wayland_idle::IdleEvent::ScreensaverResumed => {
                tracing::info!("User activity resumed — killing screensaver");
                self.kill_idle_screensaver();
                // Cancel pending lock timer
                if let Some(handle) = self.lock_timer_handle.take() {
                    handle.abort();
                    tracing::debug!("Lock timer cancelled");
                }
                Task::none()
            }
            wayland_idle::IdleEvent::LockIdle => {
                // Fallback: Wayland lock notification (may not fire reliably
                // when screensaver window resets idle timer)
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
    #[allow(clippy::needless_pass_by_value)] // Elm architecture message pattern
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

    /// Lock the screen via logind D-Bus (triggers COSMIC greeter)
    ///
    /// Note: In-process ext-session-lock-v1 is not viable because acquiring
    /// the lock disrupts the main app's Wayland connection (broken pipe),
    /// crashing the app while the lock is held. A separate binary would be
    /// needed for native session lock; for now we use loginctl lock-session.
    #[allow(clippy::unused_self)] // Method pattern; may use self in future lock strategies
    fn lock_screen(&mut self) -> Task<Message> {
        tracing::info!("Locking screen via logind D-Bus");
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
            .map_err(|()| "Invalid file path".to_string())?;

        Ok(path.to_string_lossy().to_string())
    }

    /// Build a timeout slider row with label, slider, and value display
    #[allow(clippy::cast_possible_wrap, clippy::cast_possible_truncation)]
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
            .map_or(0, |(i, _)| i as u32);

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
            .map_err(|()| "Invalid file path".to_string())?;

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
            .map_err(|()| "Invalid file path".to_string())?;

        crate::theme_config::ThemeConfig::import_theme(&path)
            .await
            .map_err(|e| e.to_string())
    }

    /// View for the Visuals page (themes)
    fn view_visuals_page(&self) -> Element<'_, Message> {
        let spacing = cosmic::theme::spacing();

        let mut column = widget::column()
            .spacing(spacing.space_m)
            .padding(spacing.space_m)
            .push(widget::text::title2(fl!("visuals")))
            .push(widget::text::body(fl!("visuals-description")));

        // Insert preview banner when actively previewing
        if let Some(banner) = self.view_preview_banner() {
            column = column.push(banner);
        }

        column = column
            // Theme preview (left) + community theme dropdowns (right)
            .push(
                widget::row()
                    .spacing(spacing.space_m)
                    .push(self.view_theme_preview_panel())
                    .push(self.view_theme_selectors()),
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
                                    Message::Page(pages::Message::Visuals(
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
                                    Message::Page(pages::Message::Visuals(
                                        pages::ThemesMessage::Import,
                                    )),
                                ),
                                widget::text::body(fl!("tooltip-import")),
                                widget::tooltip::Position::Top,
                            )),
                    ),
            );

        widget::scrollable(column).into()
    }

    /// View for the Tool Sync page
    fn view_tool_sync_page(&self) -> Element<'_, Message> {
        let spacing = cosmic::theme::spacing();

        let mut section = widget::settings::section()
            .title(fl!("tool-sync"))
            .add(widget::settings::item(
                fl!("tool-sync-auto"),
                widget::toggler(self.tool_sync_config.auto_sync).on_toggle(|enabled| {
                    Message::Page(pages::Message::Visuals(pages::ThemesMessage::SetAutoSync(
                        enabled,
                    )))
                }),
            ))
            .add(widget::settings::item(
                fl!("tool-sync-ghostty"),
                widget::toggler(self.tool_sync_config.ghostty_enabled).on_toggle(|enabled| {
                    Message::Page(pages::Message::Visuals(
                        pages::ThemesMessage::SetGhosttySync(enabled),
                    ))
                }),
            ))
            .add(widget::settings::item(
                fl!("tool-sync-btop"),
                widget::toggler(self.tool_sync_config.btop_enabled).on_toggle(|enabled| {
                    Message::Page(pages::Message::Visuals(pages::ThemesMessage::SetBtopSync(
                        enabled,
                    )))
                }),
            ))
            .add(widget::settings::item(
                fl!("tool-sync-nvim"),
                widget::toggler(self.tool_sync_config.nvim_enabled).on_toggle(|enabled| {
                    Message::Page(pages::Message::Visuals(pages::ThemesMessage::SetNvimSync(
                        enabled,
                    )))
                }),
            ))
            .add(widget::settings::item(
                fl!("tool-sync-zellij"),
                widget::toggler(self.tool_sync_config.zellij_enabled).on_toggle(|enabled| {
                    Message::Page(pages::Message::Visuals(
                        pages::ThemesMessage::SetZellijSync(enabled),
                    ))
                }),
            ))
            .add(widget::settings::item(
                fl!("tool-sync-fzf"),
                widget::toggler(self.tool_sync_config.fzf_enabled).on_toggle(|enabled| {
                    Message::Page(pages::Message::Visuals(pages::ThemesMessage::SetFzfSync(
                        enabled,
                    )))
                }),
            ))
            .add(widget::settings::item(
                fl!("tool-sync-fzf-shell"),
                widget::toggler(self.tool_sync_config.fzf_shell_integration).on_toggle(|enabled| {
                    Message::Page(pages::Message::Visuals(
                        pages::ThemesMessage::SetFzfShellIntegration(enabled),
                    ))
                }),
            ))
            .add(widget::settings::item(
                fl!("tool-sync-lazygit"),
                widget::toggler(self.tool_sync_config.lazygit_enabled).on_toggle(|enabled| {
                    Message::Page(pages::Message::Visuals(
                        pages::ThemesMessage::SetLazygitSync(enabled),
                    ))
                }),
            ))
            .add(widget::settings::item(
                fl!("tool-sync-hooks"),
                widget::toggler(self.tool_sync_config.hooks_enabled).on_toggle(|enabled| {
                    Message::Page(pages::Message::Visuals(
                        pages::ThemesMessage::SetHooksEnabled(enabled),
                    ))
                }),
            ));

        // Sync button + status row
        let mut sync_row = widget::row().spacing(spacing.space_m).push(widget::tooltip(
            widget::button::suggested(fl!("tool-sync-now")).on_press(Message::Page(
                pages::Message::Visuals(pages::ThemesMessage::SyncTools),
            )),
            widget::text::body(fl!("tool-sync-description")),
            widget::tooltip::Position::Top,
        ));

        if let Some(ref status) = self.tool_sync_status {
            sync_row = sync_row.push(widget::text::body(status));
        }

        section = section.add(sync_row);

        let column = widget::column()
            .spacing(spacing.space_m)
            .padding(spacing.space_m)
            .push(widget::text::title2(fl!("tool-sync")))
            .push(widget::text::body(fl!("tool-sync-description")))
            .push(section);

        widget::scrollable(column).into()
    }

    /// Compact theme mockup showing the active (or previewed) theme colors
    #[allow(clippy::too_many_lines, clippy::unused_self)]
    fn view_theme_preview_panel(&self) -> Element<'_, Message> {
        use cosmic::iced::Length;

        let theme = cosmic::theme::active();
        let palette = theme.cosmic();
        let accent = cosmic::iced::Color::from(palette.accent_color());
        let background = cosmic::iced::Color::from(palette.bg_color());
        let text_color = cosmic::iced::Color::from(palette.on_bg_color());

        widget::container(
            widget::column()
                .spacing(5)
                .padding(8)
                .push(
                    widget::container(widget::Space::new(Length::Fill, Length::Fixed(8.0))).class(
                        cosmic::theme::Container::custom(move |_| widget::container::Style {
                            background: Some(cosmic::iced::Background::Color(accent)),
                            border: cosmic::iced::Border {
                                radius: 2.0.into(),
                                ..Default::default()
                            },
                            ..Default::default()
                        }),
                    ),
                )
                .push(
                    widget::container(widget::Space::new(Length::Fixed(130.0), Length::Fixed(5.0)))
                        .class(cosmic::theme::Container::custom(move |_| {
                            widget::container::Style {
                                background: Some(cosmic::iced::Background::Color(
                                    cosmic::iced::Color {
                                        a: 0.7,
                                        ..text_color
                                    },
                                )),
                                border: cosmic::iced::Border {
                                    radius: 2.0.into(),
                                    ..Default::default()
                                },
                                ..Default::default()
                            }
                        })),
                )
                .push(
                    widget::container(widget::Space::new(Length::Fixed(90.0), Length::Fixed(5.0)))
                        .class(cosmic::theme::Container::custom(move |_| {
                            widget::container::Style {
                                background: Some(cosmic::iced::Background::Color(
                                    cosmic::iced::Color {
                                        a: 0.5,
                                        ..text_color
                                    },
                                )),
                                border: cosmic::iced::Border {
                                    radius: 2.0.into(),
                                    ..Default::default()
                                },
                                ..Default::default()
                            }
                        })),
                )
                .push(
                    widget::container(widget::Space::new(Length::Fixed(110.0), Length::Fixed(5.0)))
                        .class(cosmic::theme::Container::custom(move |_| {
                            widget::container::Style {
                                background: Some(cosmic::iced::Background::Color(
                                    cosmic::iced::Color {
                                        a: 0.4,
                                        ..text_color
                                    },
                                )),
                                border: cosmic::iced::Border {
                                    radius: 2.0.into(),
                                    ..Default::default()
                                },
                                ..Default::default()
                            }
                        })),
                )
                .push(
                    widget::container(widget::Space::new(Length::Fixed(50.0), Length::Fixed(10.0)))
                        .class(cosmic::theme::Container::custom(move |_| {
                            widget::container::Style {
                                background: Some(cosmic::iced::Background::Color(
                                    cosmic::iced::Color { a: 0.8, ..accent },
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
        .width(Length::Fixed(200.0))
        .height(Length::Fixed(130.0))
        .class(cosmic::theme::Container::custom(move |_| {
            widget::container::Style {
                background: Some(cosmic::iced::Background::Color(background)),
                border: cosmic::iced::Border {
                    radius: 8.0.into(),
                    width: 2.0,
                    color: accent,
                },
                ..Default::default()
            }
        }))
        .into()
    }

    /// Community theme selectors with dark and light dropdowns
    fn view_theme_selectors(&self) -> Element<'_, Message> {
        let previewing_id = self.theme_preview_backup.as_ref().map(|b| b.previewing_id);

        let dark = crate::bundled_themes::dark_themes();
        let dark_names: Vec<String> = dark.iter().map(|(m, _)| m.name.clone()).collect();
        let dark_selected = previewing_id.and_then(|id| {
            dark.iter()
                .position(|(m, _)| crate::theme_config::ThemeId::Bundled(m.index) == id)
        });
        let dark_indices: Vec<usize> = dark.iter().map(|(m, _)| m.index).collect();
        let dark_dropdown = widget::dropdown(dark_names, dark_selected, move |idx| {
            let registry_index = dark_indices[idx];
            Message::Page(pages::Message::Visuals(pages::ThemesMessage::PreviewTheme(
                crate::theme_config::ThemeId::Bundled(registry_index),
            )))
        });

        let light = crate::bundled_themes::light_themes();
        let light_names: Vec<String> = light.iter().map(|(m, _)| m.name.clone()).collect();
        let light_selected = previewing_id.and_then(|id| {
            light
                .iter()
                .position(|(m, _)| crate::theme_config::ThemeId::Bundled(m.index) == id)
        });
        let light_indices: Vec<usize> = light.iter().map(|(m, _)| m.index).collect();
        let light_dropdown = widget::dropdown(light_names, light_selected, move |idx| {
            let registry_index = light_indices[idx];
            Message::Page(pages::Message::Visuals(pages::ThemesMessage::PreviewTheme(
                crate::theme_config::ThemeId::Bundled(registry_index),
            )))
        });

        widget::settings::section()
            .title(fl!("community-themes"))
            .add(widget::settings::item(
                fl!("community-themes-dark"),
                dark_dropdown,
            ))
            .add(widget::settings::item(
                fl!("community-themes-light"),
                light_dropdown,
            ))
            .into()
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
                        pages::Message::Visuals(pages::ThemesMessage::CancelPreview),
                    )),
                )
                .push(
                    widget::button::suggested(fl!("theme-apply")).on_press(Message::Page(
                        pages::Message::Visuals(pages::ThemesMessage::ConfirmPreview),
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
    #[allow(clippy::cognitive_complexity, clippy::too_many_lines)]
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
            // Session Lock section
            .push(
                widget::settings::section()
                    .title(fl!("screensaver-session-lock"))
                    .add(widget::settings::item(
                        fl!("screensaver-session-lock-native"),
                        widget::toggler(cfg.session_lock).on_toggle(|enabled| {
                            Message::Page(pages::Message::Screensaver(
                                pages::ScreensaverMessage::SetSessionLock(enabled),
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
