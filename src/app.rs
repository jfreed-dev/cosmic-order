// SPDX-License-Identifier: GPL-3.0-only

//! Main application module
//!
//! Implements the COSMIC Application trait and handles message routing.

use cosmic::app::{Core, Task};
use cosmic::widget::{self, nav_bar};
use cosmic::{Application, Apply, Element};

use cosmic::cosmic_theme::palette::{Srgb, Srgba};
use cosmic::cosmic_theme::{CornerRadii, ThemeBuilder};
use cosmic_config::CosmicConfigEntry;

use crate::bundled_themes::ThemeSnapshot;
use crate::compositor;
use crate::config::Config;
use crate::fl;
use crate::inhibit;
use crate::pages::{self, PageId, WizardMessage};
use crate::power;
use crate::screensaver_config::ScreensaverConfig;
use crate::sleep_lock;
use crate::theme_config::{ThemeConfig, ThemePreviewState};
use crate::tool_sync::ToolSyncConfig;
use crate::wayland_idle;
use std::path::PathBuf;

/// Wizard step identifiers
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WizardStep {
    Base,
    Colors,
    Appearance,
    Save,
}

impl WizardStep {
    const fn index(self) -> u32 {
        match self {
            Self::Base => 1,
            Self::Colors => 2,
            Self::Appearance => 3,
            Self::Save => 4,
        }
    }

    fn name(self) -> String {
        match self {
            Self::Base => fl!("wizard-step-base"),
            Self::Colors => fl!("wizard-step-colors"),
            Self::Appearance => fl!("wizard-step-appearance"),
            Self::Save => fl!("wizard-step-save"),
        }
    }

    const fn next(self) -> Option<Self> {
        match self {
            Self::Base => Some(Self::Colors),
            Self::Colors => Some(Self::Appearance),
            Self::Appearance => Some(Self::Save),
            Self::Save => None,
        }
    }

    const fn prev(self) -> Option<Self> {
        match self {
            Self::Base => None,
            Self::Colors => Some(Self::Base),
            Self::Appearance => Some(Self::Colors),
            Self::Save => Some(Self::Appearance),
        }
    }
}

/// Corner radii presets (i18n key, xs, s, m+)
const CORNER_PRESETS: &[(&str, [f32; 4], [f32; 4], [f32; 4])] = &[
    ("wizard-corners-sharp", [0.0; 4], [0.0; 4], [0.0; 4]),
    ("wizard-corners-subtle", [2.0; 4], [4.0; 4], [4.0; 4]),
    ("wizard-corners-rounded", [2.0; 4], [8.0; 4], [8.0; 4]),
    (
        "wizard-corners-very-rounded",
        [4.0; 4],
        [12.0; 4],
        [16.0; 4],
    ),
];

/// Tracks wizard step and working theme state
struct WizardState {
    step: WizardStep,
    /// Working copy of the theme builder (mutated as user customizes)
    builder: ThemeBuilder,
    /// Snapshot of the theme before wizard opened (for cancel/restore)
    snapshot: ThemeSnapshot,
    /// Whether working theme is dark
    is_dark: bool,
    /// User-entered theme name
    name: String,
    /// Accent color hex string (bound to text input)
    accent_hex: String,
    /// Background color hex string (bound to text input)
    bg_hex: String,
    /// Whether custom background is enabled
    bg_override: bool,
    /// Outer window gap
    outer_gap: u32,
    /// Inner window gap
    inner_gap: u32,
    /// Active window hint size
    active_hint: u32,
    /// Corner radii preset index
    corner_preset: usize,
    /// Frosted glass effect
    is_frosted: bool,
}

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
    /// Cached text content of the selected logo file (for preview display)
    logo_preview_text: String,
    /// Active theme creation wizard state (None when wizard is closed)
    wizard_state: Option<WizardState>,
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
        let logo_preview_text = Self::load_logo_text(&screensaver_config.logo_file);
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
            logo_preview_text,
            wizard_state: None,
        };

        (app, Task::none())
    }

    fn nav_model(&self) -> Option<&nav_bar::Model> {
        Some(&self.nav_model)
    }

    #[allow(clippy::cognitive_complexity)]
    fn on_nav_select(&mut self, id: nav_bar::Id) -> Task<Message> {
        // Auto-cancel wizard when navigating away
        if let Some(wiz) = self.wizard_state.take() {
            if let Err(e) = crate::bundled_themes::restore_theme(&wiz.snapshot) {
                tracing::error!("Failed to restore theme on nav (wizard): {e}");
            }
            self.theme_config = ThemeConfig::load();
            tracing::info!("Wizard auto-cancelled on navigation");
        }

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

    #[allow(clippy::cognitive_complexity)]
    fn on_app_exit(&mut self) -> Option<Self::Message> {
        // Restore theme if wizard was active
        if let Some(wiz) = self.wizard_state.take()
            && let Err(e) = crate::bundled_themes::restore_theme(&wiz.snapshot)
        {
            tracing::error!("Failed to restore theme on exit (wizard): {e}");
        }

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
    /// Load the text content of a logo file for preview display
    fn load_logo_text(path: &str) -> String {
        if path.is_empty() {
            return String::new();
        }
        std::fs::read_to_string(path).unwrap_or_default()
    }

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
            pages::ThemesMessage::Wizard(msg) => self.handle_wizard_message(msg),
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
                self.logo_preview_text = Self::load_logo_text(&path);
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
                        self.logo_preview_text = Self::load_logo_text(path);
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

    /// Handle theme creation wizard messages
    #[allow(clippy::cognitive_complexity, clippy::too_many_lines)]
    fn handle_wizard_message(&mut self, message: WizardMessage) -> Task<Message> {
        match message {
            WizardMessage::Open => {
                // Cancel any active preview first
                if let Some(backup) = self.theme_preview_backup.take() {
                    if let Some(snapshot) = &backup.snapshot
                        && let Err(e) = crate::bundled_themes::restore_theme(snapshot)
                    {
                        tracing::error!("Failed to restore preview before wizard: {e}");
                    }
                    self.theme_config = backup.config;
                }

                let snapshot = match crate::bundled_themes::snapshot_current_theme() {
                    Ok(s) => s,
                    Err(e) => {
                        tracing::error!("Failed to snapshot theme for wizard: {e}");
                        return Task::none();
                    }
                };

                let builder = snapshot.builder.clone();
                let is_dark = snapshot.is_dark;

                // Extract current values from builder
                let accent_hex = builder
                    .accent
                    .map_or_else(|| "#63D0DE".to_string(), |c| srgb_to_hex(&c));
                let bg_hex = builder
                    .bg_color
                    .map_or_else(String::new, |c| srgba_to_hex(&c));
                let bg_override = builder.bg_color.is_some();
                let (outer_gap, inner_gap) = builder.gaps;
                let active_hint = builder.active_hint;
                let corner_preset = detect_corner_preset(&builder.corner_radii);
                let is_frosted = builder.is_frosted;

                self.wizard_state = Some(WizardState {
                    step: WizardStep::Base,
                    builder,
                    snapshot,
                    is_dark,
                    name: "My Custom Theme".to_string(),
                    accent_hex,
                    bg_hex,
                    bg_override,
                    outer_gap,
                    inner_gap,
                    active_hint,
                    corner_preset,
                    is_frosted,
                });
                tracing::info!("Theme wizard opened");
                Task::none()
            }
            WizardMessage::Close => {
                if let Some(wiz) = self.wizard_state.take() {
                    if let Err(e) = crate::bundled_themes::restore_theme(&wiz.snapshot) {
                        tracing::error!("Failed to restore theme on wizard cancel: {e}");
                    }
                    self.theme_config = ThemeConfig::load();
                    tracing::info!("Theme wizard cancelled, restored previous theme");
                }
                Task::none()
            }
            WizardMessage::NextStep => {
                if let Some(ref mut wiz) = self.wizard_state
                    && let Some(next) = wiz.step.next()
                {
                    wiz.step = next;
                }
                Task::none()
            }
            WizardMessage::PrevStep => {
                if let Some(ref mut wiz) = self.wizard_state
                    && let Some(prev) = wiz.step.prev()
                {
                    wiz.step = prev;
                }
                Task::none()
            }
            WizardMessage::SetBaseTheme(index) => {
                if let Some(ref mut wiz) = self.wizard_state {
                    // usize::MAX = "Current Theme" — restore from wizard snapshot
                    if index == usize::MAX {
                        if let Err(e) = crate::bundled_themes::restore_theme(&wiz.snapshot) {
                            tracing::error!("Failed to restore snapshot in wizard: {e}");
                            return Task::none();
                        }
                    } else if let Err(e) = crate::bundled_themes::apply_bundled_theme(index) {
                        tracing::error!("Failed to apply base theme in wizard: {e}");
                        return Task::none();
                    }
                    // Re-read the builder from config
                    if let Ok(new_builder) = Self::read_current_builder() {
                        wiz.is_dark = new_builder.palette.is_dark();
                        wiz.accent_hex = new_builder
                            .accent
                            .map_or_else(|| "#63D0DE".to_string(), |c| srgb_to_hex(&c));
                        wiz.bg_hex = new_builder
                            .bg_color
                            .map_or_else(String::new, |c| srgba_to_hex(&c));
                        wiz.bg_override = new_builder.bg_color.is_some();
                        wiz.outer_gap = new_builder.gaps.0;
                        wiz.inner_gap = new_builder.gaps.1;
                        wiz.active_hint = new_builder.active_hint;
                        wiz.corner_preset = detect_corner_preset(&new_builder.corner_radii);
                        wiz.is_frosted = new_builder.is_frosted;
                        wiz.builder = new_builder;
                    }
                    self.theme_config = ThemeConfig::load();
                }
                Task::none()
            }
            WizardMessage::SetDarkMode(is_dark) => {
                if let Some(ref mut wiz) = self.wizard_state {
                    if let Err(e) = ThemeConfig::set_dark_mode(is_dark) {
                        tracing::error!("Failed to set dark mode in wizard: {e}");
                    }
                    wiz.is_dark = is_dark;
                    // Re-read the builder from the new mode's config
                    if let Ok(new_builder) = Self::read_current_builder() {
                        wiz.builder = new_builder;
                    }
                    self.theme_config = ThemeConfig::load();
                }
                Task::none()
            }
            WizardMessage::SetAccentHex(hex) => {
                if let Some(ref mut wiz) = self.wizard_state {
                    wiz.accent_hex.clone_from(&hex);
                    if let Some(color) = parse_hex_to_srgb(&hex) {
                        wiz.builder.accent = Some(color);
                        if let Err(e) = ThemeConfig::write_builder(&wiz.builder, wiz.is_dark) {
                            tracing::error!("Failed to write accent in wizard: {e}");
                        }
                        self.theme_config = ThemeConfig::load();
                    }
                }
                Task::none()
            }
            WizardMessage::SetAccentPreset(packed) => {
                if let Some(ref mut wiz) = self.wizard_state {
                    let color = unpack_rgb(packed);
                    wiz.accent_hex = srgb_to_hex(&color);
                    wiz.builder.accent = Some(color);
                    if let Err(e) = ThemeConfig::write_builder(&wiz.builder, wiz.is_dark) {
                        tracing::error!("Failed to write accent preset in wizard: {e}");
                    }
                    self.theme_config = ThemeConfig::load();
                }
                Task::none()
            }
            WizardMessage::SetBgHex(hex) => {
                if let Some(ref mut wiz) = self.wizard_state {
                    wiz.bg_hex.clone_from(&hex);
                    if let Some(color) = parse_hex_to_srgb(&hex) {
                        wiz.builder.bg_color =
                            Some(Srgba::new(color.red, color.green, color.blue, 1.0));
                        if let Err(e) = ThemeConfig::write_builder(&wiz.builder, wiz.is_dark) {
                            tracing::error!("Failed to write bg color in wizard: {e}");
                        }
                        self.theme_config = ThemeConfig::load();
                    }
                }
                Task::none()
            }
            WizardMessage::SetBgOverride(enabled) => {
                if let Some(ref mut wiz) = self.wizard_state {
                    wiz.bg_override = enabled;
                    if !enabled {
                        wiz.builder.bg_color = None;
                        wiz.bg_hex.clear();
                        if let Err(e) = ThemeConfig::write_builder(&wiz.builder, wiz.is_dark) {
                            tracing::error!("Failed to clear bg override in wizard: {e}");
                        }
                        self.theme_config = ThemeConfig::load();
                    }
                }
                Task::none()
            }
            WizardMessage::SetOuterGap(gap) => {
                if let Some(ref mut wiz) = self.wizard_state {
                    wiz.outer_gap = gap;
                    wiz.builder.gaps = (gap, wiz.inner_gap);
                    if let Err(e) = ThemeConfig::write_builder(&wiz.builder, wiz.is_dark) {
                        tracing::error!("Failed to write outer gap in wizard: {e}");
                    }
                }
                Task::none()
            }
            WizardMessage::SetInnerGap(gap) => {
                if let Some(ref mut wiz) = self.wizard_state {
                    wiz.inner_gap = gap;
                    wiz.builder.gaps = (wiz.outer_gap, gap);
                    if let Err(e) = ThemeConfig::write_builder(&wiz.builder, wiz.is_dark) {
                        tracing::error!("Failed to write inner gap in wizard: {e}");
                    }
                }
                Task::none()
            }
            WizardMessage::SetActiveHint(hint) => {
                if let Some(ref mut wiz) = self.wizard_state {
                    wiz.active_hint = hint;
                    wiz.builder.active_hint = hint;
                    if let Err(e) = ThemeConfig::write_builder(&wiz.builder, wiz.is_dark) {
                        tracing::error!("Failed to write active hint in wizard: {e}");
                    }
                }
                Task::none()
            }
            WizardMessage::SetCornerPreset(preset_idx) => {
                if let Some(ref mut wiz) = self.wizard_state
                    && let Some(&(_, xs, s, m)) = CORNER_PRESETS.get(preset_idx)
                {
                    wiz.corner_preset = preset_idx;
                    wiz.builder.corner_radii = CornerRadii {
                        radius_0: [0.0; 4],
                        radius_xs: xs,
                        radius_s: s,
                        radius_m: m,
                        radius_l: m,
                        radius_xl: m,
                    };
                    if let Err(e) = ThemeConfig::write_builder(&wiz.builder, wiz.is_dark) {
                        tracing::error!("Failed to write corner radii in wizard: {e}");
                    }
                }
                Task::none()
            }
            WizardMessage::SetFrosted(enabled) => {
                if let Some(ref mut wiz) = self.wizard_state {
                    wiz.is_frosted = enabled;
                    wiz.builder.is_frosted = enabled;
                    if let Err(e) = ThemeConfig::write_builder(&wiz.builder, wiz.is_dark) {
                        tracing::error!("Failed to write frosted in wizard: {e}");
                    }
                }
                Task::none()
            }
            WizardMessage::SetName(name) => {
                if let Some(ref mut wiz) = self.wizard_state {
                    wiz.name = name;
                }
                Task::none()
            }
            WizardMessage::Export => {
                let Some(ref wiz) = self.wizard_state else {
                    return Task::none();
                };
                let builder = wiz.builder.clone();
                let name = wiz.name.clone();
                cosmic::task::future(async move {
                    let result = Self::run_wizard_export(builder, &name).await;
                    Message::Page(pages::Message::Visuals(pages::ThemesMessage::Wizard(
                        WizardMessage::ExportComplete(result),
                    )))
                })
            }
            WizardMessage::ExportComplete(result) => {
                match &result {
                    Ok(path) => tracing::info!("Wizard theme exported to: {path}"),
                    Err(e) => {
                        if e == "cancelled" {
                            tracing::debug!("Wizard export cancelled");
                        } else {
                            tracing::error!("Wizard export failed: {e}");
                        }
                    }
                }
                Task::none()
            }
            WizardMessage::Apply => {
                // Keep the current theme (don't restore snapshot), close wizard
                self.wizard_state = None;
                self.theme_config = ThemeConfig::load();
                tracing::info!("Wizard theme applied");
                Task::none()
            }
        }
    }

    /// Read the current `ThemeBuilder` from cosmic-config
    fn read_current_builder() -> Result<ThemeBuilder, crate::theme_config::ThemeError> {
        use crate::theme_config::ThemeError;
        use cosmic::cosmic_theme::ThemeMode;

        let mode_config =
            ThemeMode::config().map_err(|e| ThemeError::ConfigAccess(e.to_string()))?;
        let mode = match ThemeMode::get_entry(&mode_config) {
            Ok(m) | Err((_, m)) => m,
        };

        let builder_config = if mode.is_dark {
            ThemeBuilder::dark_config()
        } else {
            ThemeBuilder::light_config()
        }
        .map_err(|e| ThemeError::ConfigAccess(e.to_string()))?;

        Ok(match ThemeBuilder::get_entry(&builder_config) {
            Ok(b) | Err((_, b)) => b,
        })
    }

    /// Export a wizard theme builder as RON via file save dialog
    async fn run_wizard_export(builder: ThemeBuilder, name: &str) -> Result<String, String> {
        use cosmic::dialog::file_chooser;

        let sanitized = name.to_lowercase().replace(' ', "-");
        let filename = format!("{sanitized}.ron");

        let dialog = file_chooser::save::Dialog::new()
            .title(fl!("wizard-export"))
            .file_name(filename)
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

        let pretty = ron::ser::PrettyConfig::default();
        let serialized = ron::ser::to_string_pretty(&builder, pretty)
            .map_err(|e| format!("Serialize error: {e}"))?;

        tokio::fs::write(&path, &serialized)
            .await
            .map_err(|e| format!("Write error: {e}"))?;

        Ok(path.to_string_lossy().to_string())
    }

    /// View for the Visuals page (themes)
    fn view_visuals_page(&self) -> Element<'_, Message> {
        // Show wizard view when active
        if self.wizard_state.is_some() {
            return self.view_wizard();
        }

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
            // Theme preview (centered above) + community theme dropdowns (below)
            .push(
                widget::container(self.view_theme_preview_panel())
                    .align_x(cosmic::iced::alignment::Horizontal::Center)
                    .width(cosmic::iced::Length::Fill),
            )
            .push(self.view_theme_selectors())
            // Export & Import + Create Theme
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
                    )
                    .add(
                        widget::button::suggested(fl!("wizard-create-theme")).on_press(
                            Message::Page(pages::Message::Visuals(pages::ThemesMessage::Wizard(
                                WizardMessage::Open,
                            ))),
                        ),
                    ),
            );

        widget::scrollable(column).into()
    }

    /// Wizard main view — orchestrates steps, navigation, and preview panel
    #[allow(clippy::too_many_lines)]
    fn view_wizard(&self) -> Element<'_, Message> {
        let Some(ref wiz) = self.wizard_state else {
            return widget::text::body("").into();
        };

        let spacing = cosmic::theme::spacing();

        // Step indicator
        let step_name = wiz.step.name();
        let indicator = fl!(
            "wizard-step-indicator",
            step = wiz.step.index().to_string(),
            total = "4",
            name = step_name
        );

        // Step content
        let step_content: Element<'_, Message> = match wiz.step {
            WizardStep::Base => self.view_wizard_step_base(wiz),
            WizardStep::Colors => self.view_wizard_step_colors(wiz),
            WizardStep::Appearance => self.view_wizard_step_appearance(wiz),
            WizardStep::Save => self.view_wizard_step_save(wiz),
        };

        // Navigation row
        let mut nav_row = widget::row().spacing(spacing.space_m);

        // Cancel button (always visible)
        nav_row = nav_row.push(
            widget::button::standard(fl!("cancel")).on_press(Message::Page(
                pages::Message::Visuals(pages::ThemesMessage::Wizard(WizardMessage::Close)),
            )),
        );

        // Back button (hidden on first step)
        if wiz.step.prev().is_some() {
            nav_row = nav_row.push(widget::button::standard(fl!("wizard-back")).on_press(
                Message::Page(pages::Message::Visuals(pages::ThemesMessage::Wizard(
                    WizardMessage::PrevStep,
                ))),
            ));
        }

        // Next / Apply / Export buttons depending on step
        if wiz.step == WizardStep::Save {
            nav_row = nav_row
                .push(
                    widget::button::standard(fl!("wizard-export")).on_press(Message::Page(
                        pages::Message::Visuals(pages::ThemesMessage::Wizard(
                            WizardMessage::Export,
                        )),
                    )),
                )
                .push(
                    widget::button::suggested(fl!("wizard-apply")).on_press(Message::Page(
                        pages::Message::Visuals(pages::ThemesMessage::Wizard(WizardMessage::Apply)),
                    )),
                );
        } else {
            nav_row = nav_row.push(widget::button::suggested(fl!("wizard-next")).on_press(
                Message::Page(pages::Message::Visuals(pages::ThemesMessage::Wizard(
                    WizardMessage::NextStep,
                ))),
            ));
        }

        let column = widget::column()
            .spacing(spacing.space_m)
            .padding(spacing.space_m)
            .push(widget::text::title2(fl!("wizard-title")))
            .push(widget::text::body(indicator))
            .push(
                widget::row()
                    .spacing(spacing.space_m)
                    .push(step_content)
                    .push(self.view_theme_preview_panel()),
            )
            .push(nav_row);

        widget::scrollable(column).into()
    }

    /// Wizard Step 1: Base Theme selection
    #[allow(clippy::unused_self)]
    fn view_wizard_step_base<'a>(&'a self, wiz: &'a WizardState) -> Element<'a, Message> {
        // Build dropdown: "Current Theme" + all bundled theme names
        let themes = crate::bundled_themes::all_themes();
        let mut names: Vec<String> = vec![fl!("wizard-current-theme")];
        names.extend(themes.iter().map(|(m, _)| m.name.clone()));

        // No selection by default (user picks from list)
        let base_dropdown = widget::dropdown(names, None::<usize>, move |idx| {
            if idx == 0 {
                // "Current Theme" selected — reload from snapshot
                Message::Page(pages::Message::Visuals(pages::ThemesMessage::Wizard(
                    WizardMessage::SetBaseTheme(usize::MAX),
                )))
            } else {
                let registry_idx = themes[idx - 1].0.index;
                Message::Page(pages::Message::Visuals(pages::ThemesMessage::Wizard(
                    WizardMessage::SetBaseTheme(registry_idx),
                )))
            }
        });

        widget::settings::section()
            .title(fl!("wizard-step-base"))
            .add(widget::settings::item(
                fl!("wizard-start-from"),
                base_dropdown,
            ))
            .add(widget::settings::item(
                fl!("wizard-dark-mode"),
                widget::toggler(wiz.is_dark).on_toggle(|dark| {
                    Message::Page(pages::Message::Visuals(pages::ThemesMessage::Wizard(
                        WizardMessage::SetDarkMode(dark),
                    )))
                }),
            ))
            .apply(widget::container)
            .width(cosmic::iced::Length::Fill)
            .into()
    }

    /// Wizard Step 2: Colors (accent + background)
    #[allow(clippy::too_many_lines)]
    fn view_wizard_step_colors<'a>(&'a self, wiz: &'a WizardState) -> Element<'a, Message> {
        let spacing = cosmic::theme::spacing();

        // Accent hex input
        let accent_input =
            widget::text_input(fl!("wizard-accent-hex"), &wiz.accent_hex).on_input(|hex| {
                Message::Page(pages::Message::Visuals(pages::ThemesMessage::Wizard(
                    WizardMessage::SetAccentHex(hex),
                )))
            });

        // Accent presets — COSMIC colors (dark-aware)
        let cosmic_accents: &[(u8, u8, u8)] = if wiz.is_dark {
            &[
                (99, 208, 222),  // Blue
                (129, 137, 236), // Indigo
                (173, 131, 220), // Purple
                (215, 129, 194), // Pink
                (230, 116, 118), // Red
                (230, 150, 92),  // Orange
                (222, 199, 76),  // Yellow
                (95, 199, 128),  // Green
                (168, 153, 152), // Warm Grey
            ]
        } else {
            &[
                (38, 133, 203),  // Blue
                (104, 96, 202),  // Indigo
                (147, 90, 195),  // Purple
                (192, 88, 160),  // Pink
                (207, 73, 79),   // Red
                (207, 109, 42),  // Orange
                (193, 161, 26),  // Yellow
                (46, 163, 84),   // Green
                (143, 116, 115), // Warm Grey
            ]
        };

        let mut cosmic_row = widget::row().spacing(spacing.space_xxs);
        for &(r, g, b) in cosmic_accents {
            cosmic_row = cosmic_row.push(self.view_accent_swatch(r, g, b, &wiz.accent_hex));
        }

        // Cosmictron accent presets
        let cosmictron_accents: &[(u8, u8, u8)] = &[
            (92, 160, 207),  // Blue
            (163, 131, 97),  // Brown
            (97, 173, 131),  // Green
            (207, 131, 173), // Pink
            (152, 120, 191), // Purple
            (199, 109, 109), // Red
            (92, 184, 179),  // Teal
            (214, 196, 101), // Yellow
        ];

        let mut cosmictron_row = widget::row().spacing(spacing.space_xxs);
        for &(r, g, b) in cosmictron_accents {
            cosmictron_row = cosmictron_row.push(self.view_accent_swatch(r, g, b, &wiz.accent_hex));
        }

        let mut section = widget::settings::section()
            .title(fl!("wizard-step-colors"))
            .add(widget::settings::item(
                fl!("wizard-accent-color"),
                accent_input,
            ))
            .add(cosmic_row)
            .add(cosmictron_row);

        // Background override
        section = section.add(widget::settings::item(
            fl!("wizard-bg-override"),
            widget::toggler(wiz.bg_override).on_toggle(|enabled| {
                Message::Page(pages::Message::Visuals(pages::ThemesMessage::Wizard(
                    WizardMessage::SetBgOverride(enabled),
                )))
            }),
        ));

        if wiz.bg_override {
            let bg_input =
                widget::text_input(fl!("wizard-accent-hex"), &wiz.bg_hex).on_input(|hex| {
                    Message::Page(pages::Message::Visuals(pages::ThemesMessage::Wizard(
                        WizardMessage::SetBgHex(hex),
                    )))
                });
            section = section.add(widget::settings::item(fl!("wizard-bg-color"), bg_input));
        }

        widget::container(section)
            .width(cosmic::iced::Length::Fill)
            .into()
    }

    /// Build a single accent color swatch button
    #[allow(clippy::cast_lossless, clippy::unused_self)]
    fn view_accent_swatch(&self, r: u8, g: u8, b: u8, current_hex: &str) -> Element<'_, Message> {
        let packed = pack_rgb(r, g, b);
        let color = cosmic::iced::Color::from_rgb8(r, g, b);

        // Check if this swatch matches current accent
        let swatch_hex = format!("#{r:02X}{g:02X}{b:02X}");
        let is_selected = current_hex.eq_ignore_ascii_case(&swatch_hex);

        let border = if is_selected {
            cosmic::iced::Border {
                radius: 4.0.into(),
                width: 2.0,
                color: cosmic::iced::Color::WHITE,
            }
        } else {
            cosmic::iced::Border {
                radius: 4.0.into(),
                ..Default::default()
            }
        };

        widget::button::custom(
            widget::container(widget::Space::new(
                cosmic::iced::Length::Fixed(22.0),
                cosmic::iced::Length::Fixed(22.0),
            ))
            .class(cosmic::theme::Container::custom(move |_| {
                widget::container::Style {
                    background: Some(cosmic::iced::Background::Color(color)),
                    border,
                    ..Default::default()
                }
            })),
        )
        .on_press(Message::Page(pages::Message::Visuals(
            pages::ThemesMessage::Wizard(WizardMessage::SetAccentPreset(packed)),
        )))
        .padding(0)
        .into()
    }

    /// Wizard Step 3: Appearance (gaps, hint, corners, frosted)
    #[allow(clippy::unused_self)]
    fn view_wizard_step_appearance<'a>(&'a self, wiz: &'a WizardState) -> Element<'a, Message> {
        // Corner radii preset dropdown
        let corner_names: Vec<String> = vec![
            fl!("wizard-corners-sharp"),
            fl!("wizard-corners-subtle"),
            fl!("wizard-corners-rounded"),
            fl!("wizard-corners-very-rounded"),
        ];
        let corner_dropdown = widget::dropdown(corner_names, Some(wiz.corner_preset), |idx| {
            Message::Page(pages::Message::Visuals(pages::ThemesMessage::Wizard(
                WizardMessage::SetCornerPreset(idx),
            )))
        });

        // Gap sliders (0–16 range)
        let outer_gap = wiz.outer_gap;
        let inner_gap = wiz.inner_gap;
        let active_hint = wiz.active_hint;

        let outer_slider = widget::slider(0u32..=16u32, outer_gap, |v| {
            Message::Page(pages::Message::Visuals(pages::ThemesMessage::Wizard(
                WizardMessage::SetOuterGap(v),
            )))
        });

        let inner_slider = widget::slider(0u32..=16u32, inner_gap, |v| {
            Message::Page(pages::Message::Visuals(pages::ThemesMessage::Wizard(
                WizardMessage::SetInnerGap(v),
            )))
        });

        let hint_slider = widget::slider(0u32..=10u32, active_hint, |v| {
            Message::Page(pages::Message::Visuals(pages::ThemesMessage::Wizard(
                WizardMessage::SetActiveHint(v),
            )))
        });

        widget::settings::section()
            .title(fl!("wizard-step-appearance"))
            .add(widget::settings::item(
                format!("{} ({})", fl!("wizard-outer-gap"), outer_gap),
                outer_slider,
            ))
            .add(widget::settings::item(
                format!("{} ({})", fl!("wizard-inner-gap"), inner_gap),
                inner_slider,
            ))
            .add(widget::settings::item(
                format!("{} ({})", fl!("wizard-active-hint"), active_hint),
                hint_slider,
            ))
            .add(widget::settings::item(
                fl!("wizard-corners"),
                corner_dropdown,
            ))
            .add(widget::settings::item(
                fl!("wizard-frosted"),
                widget::toggler(wiz.is_frosted).on_toggle(|enabled| {
                    Message::Page(pages::Message::Visuals(pages::ThemesMessage::Wizard(
                        WizardMessage::SetFrosted(enabled),
                    )))
                }),
            ))
            .apply(widget::container)
            .width(cosmic::iced::Length::Fill)
            .into()
    }

    /// Wizard Step 4: Name & Save
    #[allow(clippy::unused_self)]
    fn view_wizard_step_save<'a>(&'a self, wiz: &'a WizardState) -> Element<'a, Message> {
        let name_input = widget::text_input(fl!("wizard-theme-name"), &wiz.name).on_input(|name| {
            Message::Page(pages::Message::Visuals(pages::ThemesMessage::Wizard(
                WizardMessage::SetName(name),
            )))
        });

        // Summary info
        let mode_text = if wiz.is_dark {
            fl!("wizard-dark-mode")
        } else {
            "Light mode".to_string()
        };

        widget::settings::section()
            .title(fl!("wizard-step-save"))
            .add(widget::settings::item(fl!("wizard-theme-name"), name_input))
            .add(widget::text::body(format!(
                "{mode_text} | {} {} | {} {}",
                fl!("wizard-outer-gap"),
                wiz.outer_gap,
                fl!("wizard-inner-gap"),
                wiz.inner_gap,
            )))
            .apply(widget::container)
            .width(cosmic::iced::Length::Fill)
            .into()
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
        .width(Length::Fixed(300.0))
        .height(Length::Fixed(195.0))
        .class(cosmic::theme::Container::custom(move |_| {
            widget::container::Style {
                background: Some(cosmic::iced::Background::Color(background)),
                border: cosmic::iced::Border {
                    radius: 8.0.into(),
                    width: 1.0,
                    color: cosmic::iced::Color {
                        a: 0.3,
                        ..text_color
                    },
                },
                ..Default::default()
            }
        }))
        .into()
    }

    /// Community theme selectors with dark and light dropdowns
    fn view_theme_selectors(&self) -> Element<'_, Message> {
        let previewing_id = self.theme_preview_backup.as_ref().map(|b| b.previewing_id);

        let cosmic_dark = crate::bundled_themes::cosmic_dark_themes();
        let cosmic_dark_names: Vec<String> =
            cosmic_dark.iter().map(|(m, _)| m.name.clone()).collect();
        let cosmic_dark_selected = previewing_id.and_then(|id| {
            cosmic_dark
                .iter()
                .position(|(m, _)| crate::theme_config::ThemeId::Bundled(m.index) == id)
        });
        let cosmic_dark_indices: Vec<usize> = cosmic_dark.iter().map(|(m, _)| m.index).collect();
        let cosmic_dark_dropdown =
            widget::dropdown(cosmic_dark_names, cosmic_dark_selected, move |idx| {
                let registry_index = cosmic_dark_indices[idx];
                Message::Page(pages::Message::Visuals(pages::ThemesMessage::PreviewTheme(
                    crate::theme_config::ThemeId::Bundled(registry_index),
                )))
            });

        let cosmictron = crate::bundled_themes::cosmictron_dark_themes();
        let cosmictron_names: Vec<String> =
            cosmictron.iter().map(|(m, _)| m.name.clone()).collect();
        let cosmictron_selected = previewing_id.and_then(|id| {
            cosmictron
                .iter()
                .position(|(m, _)| crate::theme_config::ThemeId::Bundled(m.index) == id)
        });
        let cosmictron_indices: Vec<usize> = cosmictron.iter().map(|(m, _)| m.index).collect();
        let cosmictron_dropdown =
            widget::dropdown(cosmictron_names, cosmictron_selected, move |idx| {
                let registry_index = cosmictron_indices[idx];
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
                cosmic_dark_dropdown,
            ))
            .add(widget::settings::item(
                fl!("community-themes-cosmictron"),
                cosmictron_dropdown,
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
    fn view_screensaver_page(&self) -> Element<'_, Message> {
        let spacing = cosmic::theme::spacing();
        let column = widget::column()
            .spacing(spacing.space_m)
            .padding(spacing.space_m)
            .push(widget::text::title2(fl!("screensaver")))
            .push(widget::text::body(fl!("screensaver-description")))
            .push(self.view_screensaver_preview_section())
            .push(self.view_screensaver_settings_section());
        widget::scrollable(column).into()
    }

    /// Preview sub-section: logo text preview, Save & Test, logo selector, effects, timeouts
    #[allow(clippy::too_many_lines)]
    fn view_screensaver_preview_section(&self) -> Element<'_, Message> {
        let spacing = cosmic::theme::spacing();
        let cfg = &self.screensaver_config;

        // --- Logo text preview container ---
        let logo_preview: Element<'_, Message> = if self.logo_preview_text.is_empty() {
            widget::container(widget::text::caption(fl!("screensaver-no-logo")))
                .padding(spacing.space_m)
                .width(cosmic::iced::Length::Fixed(300.0))
                .height(cosmic::iced::Length::Fixed(195.0))
                .align_x(cosmic::iced::alignment::Horizontal::Center)
                .align_y(cosmic::iced::alignment::Vertical::Center)
                .class(cosmic::theme::Container::custom(|theme| {
                    let cosmic = theme.cosmic();
                    let bg = cosmic.bg_divider();
                    let on_bg = cosmic.on_bg_color();
                    widget::container::Style {
                        background: Some(cosmic::iced::Background::Color(
                            cosmic::iced::Color::from(bg),
                        )),
                        border: cosmic::iced::Border {
                            radius: 8.0.into(),
                            width: 1.0,
                            color: cosmic::iced::Color {
                                a: 0.3,
                                ..cosmic::iced::Color::from(on_bg)
                            },
                        },
                        ..Default::default()
                    }
                }))
                .into()
        } else {
            let lines = self.logo_preview_text.lines().count().max(1);
            let max_cols = self
                .logo_preview_text
                .lines()
                .map(str::len)
                .max()
                .unwrap_or(1)
                .max(1);
            let pad = f32::from(spacing.space_m) * 2.0;
            let available_height = 195.0 - pad;
            let available_width = 300.0 - pad;
            // Monospace: char width ≈ 0.6 × font size
            #[allow(clippy::cast_precision_loss)]
            let size_by_height = available_height / (lines as f32 * 1.2);
            #[allow(clippy::cast_precision_loss)]
            let size_by_width = available_width / (max_cols as f32 * 0.6);
            let font_size = size_by_height.min(size_by_width).clamp(2.0, 14.0);

            widget::container(
                widget::text(&self.logo_preview_text)
                    .size(font_size)
                    .font(cosmic::iced::Font::MONOSPACE),
            )
            .padding(spacing.space_m)
            .width(cosmic::iced::Length::Fixed(300.0))
            .height(cosmic::iced::Length::Fixed(195.0))
            .align_x(cosmic::iced::alignment::Horizontal::Center)
            .align_y(cosmic::iced::alignment::Vertical::Center)
            .class(cosmic::theme::Container::custom(|theme| {
                let cosmic = theme.cosmic();
                let bg = cosmic.bg_divider();
                let on_bg = cosmic.on_bg_color();
                widget::container::Style {
                    background: Some(cosmic::iced::Background::Color(cosmic::iced::Color::from(
                        bg,
                    ))),
                    border: cosmic::iced::Border {
                        radius: 8.0.into(),
                        width: 1.0,
                        color: cosmic::iced::Color {
                            a: 0.3,
                            ..cosmic::iced::Color::from(on_bg)
                        },
                    },
                    ..Default::default()
                }
            }))
            .into()
        };

        // --- Save & Test button ---
        let test_btn = widget::tooltip(
            widget::button::standard(fl!("screensaver-save-test")).on_press(Message::Page(
                pages::Message::Screensaver(pages::ScreensaverMessage::SaveAndTest),
            )),
            widget::text::body(fl!("tooltip-save-test")),
            widget::tooltip::Position::Top,
        );

        // --- Timeout sliders ---
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

        let centered_preview = widget::container(logo_preview)
            .align_x(cosmic::iced::alignment::Horizontal::Center)
            .width(cosmic::iced::Length::Fill);

        widget::column()
            .spacing(spacing.space_s)
            .push(widget::text::title4(fl!("screensaver-preview")))
            .push(centered_preview)
            .push(test_btn)
            // Logo selector
            .push(self.view_screensaver_logo_section())
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
            // Timeouts section
            .push(
                widget::settings::section()
                    .title(fl!("screensaver-timeouts"))
                    .add(idle_slider)
                    .add(lock_slider)
                    .add(dpms_slider),
            )
            .into()
    }

    /// Settings sub-section: status, clock, cursor & dismiss, session lock, save button
    #[allow(clippy::too_many_lines)]
    fn view_screensaver_settings_section(&self) -> Element<'_, Message> {
        let spacing = cosmic::theme::spacing();
        let cfg = &self.screensaver_config;

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

        widget::column()
            .spacing(spacing.space_s)
            .push(widget::text::title4(fl!("screensaver-settings")))
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

                action_col = action_col.push(save_btn);
                action_col
            })
            .into()
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

// --- Wizard color helpers ---

/// Pack (r, g, b) as u8 values into a single u32
#[allow(clippy::cast_lossless)]
const fn pack_rgb(r: u8, g: u8, b: u8) -> u32 {
    (r as u32) << 16 | (g as u32) << 8 | b as u32
}

/// Unpack a u32 to Srgb (0.0–1.0)
#[allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)]
fn unpack_rgb(packed: u32) -> Srgb {
    let r = ((packed >> 16) & 0xFF) as f32 / 255.0;
    let g = ((packed >> 8) & 0xFF) as f32 / 255.0;
    let b = (packed & 0xFF) as f32 / 255.0;
    Srgb::new(r, g, b)
}

/// Format an Srgb color as a hex string (#RRGGBB)
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn srgb_to_hex(c: &Srgb) -> String {
    format!(
        "#{:02X}{:02X}{:02X}",
        (c.red * 255.0) as u8,
        (c.green * 255.0) as u8,
        (c.blue * 255.0) as u8
    )
}

/// Format an Srgba color as a hex string (#RRGGBB)
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn srgba_to_hex(c: &Srgba) -> String {
    format!(
        "#{:02X}{:02X}{:02X}",
        (c.red * 255.0) as u8,
        (c.green * 255.0) as u8,
        (c.blue * 255.0) as u8
    )
}

/// Parse a hex string (#RRGGBB or RRGGBB) to Srgb
fn parse_hex_to_srgb(hex: &str) -> Option<Srgb> {
    let hex = hex.trim_start_matches('#');
    if hex.len() != 6 {
        return None;
    }
    let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
    let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
    let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
    Some(Srgb::new(
        f32::from(r) / 255.0,
        f32::from(g) / 255.0,
        f32::from(b) / 255.0,
    ))
}

/// Detect which corner preset index best matches the given radii
#[allow(clippy::float_cmp)]
fn detect_corner_preset(radii: &CornerRadii) -> usize {
    for (i, &(_, xs, s, m)) in CORNER_PRESETS.iter().enumerate() {
        if radii.radius_xs == xs && radii.radius_s == s && radii.radius_m == m {
            return i;
        }
    }
    // Default to "Rounded" if no match
    2
}
