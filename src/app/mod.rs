// SPDX-License-Identifier: GPL-3.0-only

//! Main application module
//!
//! Implements the COSMIC Application trait and handles message routing.

mod idle;
mod screensaver;
mod tool_sync_view;
mod visuals;

use cosmic::app::{Core, Task};
use cosmic::widget::{self, nav_bar};
use cosmic::{Application, Apply, Element};

use cosmic::cosmic_theme::palette::Srgba;
use cosmic::cosmic_theme::{CornerRadii, ThemeBuilder};
use cosmic_config::CosmicConfigEntry;

use crate::colors::{pack_rgb, parse_hex_to_srgb, srgb_to_hex, srgba_to_hex, unpack_rgb};

use crate::bundled_themes::ThemeSnapshot;
use crate::compositor;
use crate::config::Config;
use crate::fl;
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
pub enum WizardStep {
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
pub const CORNER_PRESETS: &[(&str, [f32; 4], [f32; 4], [f32; 4])] = &[
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
pub struct WizardState {
    pub step: WizardStep,
    /// Working copy of the theme builder (mutated as user customizes)
    pub builder: ThemeBuilder,
    /// Snapshot of the theme before wizard opened (for cancel/restore)
    pub snapshot: ThemeSnapshot,
    /// Whether working theme is dark
    pub is_dark: bool,
    /// User-entered theme name
    pub name: String,
    /// Accent color hex string (bound to text input)
    pub accent_hex: String,
    /// Background color hex string (bound to text input)
    pub bg_hex: String,
    /// Whether custom background is enabled
    pub bg_override: bool,
    /// Outer window gap
    pub outer_gap: u32,
    /// Inner window gap
    pub inner_gap: u32,
    /// Active window hint size
    pub active_hint: u32,
    /// Corner radii preset index
    pub corner_preset: usize,
    /// Frosted glass effect
    pub is_frosted: bool,
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
    /// Last observed window size, persisted on exit
    last_window_size: (u32, u32),
}

/// Application messages
#[derive(Debug, Clone)]
pub enum Message {
    /// Navigation item selected (constructed by libcosmic framework)
    #[allow(dead_code)]
    NavSelect(nav_bar::Id),
    /// Page-specific message
    Page(pages::Message),
    /// Power state updated from D-Bus subscription
    PowerStateUpdate(power::PowerState),
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
    type Flags = Option<PageId>;

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
    fn init(mut core: Core, start_page: Self::Flags) -> (Self, Task<Message>) {
        // Title shown in the COSMIC headerbar
        core.set_header_title(fl!("app-title"));

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

        // Activate the requested page (`--page`), else the saved page from config
        let active_page = start_page.unwrap_or(config.active_page);
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
        let last_window_size = (config.window_width, config.window_height);
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
            tool_sync_config: ToolSyncConfig::load(),
            tool_sync_status: None,
            native_idle_active: false,
            idle_subscription_config,
            idle_screensaver_child: None,
            session_lock_enabled,
            lock_timer_handle: None,
            logo_preview_text,
            wizard_state: None,
            last_window_size,
        };

        let init_task = if let Some(id) = app.core.main_window_id() {
            let (w, h) = last_window_size;
            if w > 0 && h > 0 {
                #[allow(clippy::cast_precision_loss)]
                cosmic::iced::window::resize(id, cosmic::iced::Size::new(w as f32, h as f32))
            } else {
                Task::none()
            }
        } else {
            Task::none()
        };

        (app, init_task)
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

    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    fn on_window_resize(&mut self, _id: cosmic::iced::window::Id, width: f32, height: f32) {
        if width > 0.0 && height > 0.0 {
            self.last_window_size = (width.round() as u32, height.round() as u32);
        }
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

        let (w, h) = self.last_window_size;
        if w > 0 && h > 0 && (self.config.window_width != w || self.config.window_height != h) {
            self.config.window_width = w;
            self.config.window_height = h;
            if let Err(e) = self.config.save() {
                tracing::warn!("Failed to persist window size: {e}");
            }
        }
        None
    }

    #[allow(clippy::cognitive_complexity)]
    fn update(&mut self, message: Self::Message) -> Task<Message> {
        match message {
            Message::NavSelect(id) => self.on_nav_select(id),
            Message::Page(page_message) => self.handle_page_message(page_message),
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

                self.power_state = Some(state);
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
        // Title is rendered by the headerbar itself via `set_header_title`
        // (set in `init`); avoid a second, oversized title widget here.
        vec![]
    }

    fn header_end(&self) -> Vec<Element<'_, Self::Message>> {
        vec![]
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
}

impl Drop for App {
    fn drop(&mut self) {
        if self.native_idle_active {
            // Safety net: restart swayidle when the app is dropped
            Self::restart_swayidle_sync();
        }
    }
}

/// Detect which corner preset index best matches the given radii
#[allow(clippy::float_cmp)]
pub fn detect_corner_preset(radii: &CornerRadii) -> usize {
    for (i, &(_, xs, s, m)) in CORNER_PRESETS.iter().enumerate() {
        if radii.radius_xs == xs && radii.radius_s == s && radii.radius_m == m {
            return i;
        }
    }
    // Default to "Rounded" if no match
    2
}
