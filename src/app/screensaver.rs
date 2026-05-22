// SPDX-License-Identifier: GPL-3.0-only

#[allow(clippy::wildcard_imports)]
use super::*;

impl App {
    /// Handle screensaver page messages
    #[allow(clippy::cognitive_complexity, clippy::too_many_lines)]
    pub(super) fn handle_screensaver_message(
        &mut self,
        message: pages::ScreensaverMessage,
    ) -> Task<Message> {
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
            pages::ScreensaverMessage::SetClockFont(font) => {
                self.screensaver_config.clock_font = font;
                Task::none()
            }
            pages::ScreensaverMessage::SetTerminal(index) => {
                let terminals = ["alacritty", "ghostty", "cosmic-term"];
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
            pages::ScreensaverMessage::SetDisableOnBattery(enabled) => {
                self.screensaver_config.disable_on_battery = enabled;
                Task::none()
            }
            pages::ScreensaverMessage::SetBatteryIdleTimeout(index) => {
                let values: [u32; 4] = [300, 600, 900, 1800];
                if let Some(&v) = values.get(index) {
                    self.screensaver_config.battery_idle_timeout = v;
                }
                Task::none()
            }
            pages::ScreensaverMessage::SetEffectsForProfile(slot, text) => {
                let cfg = &mut self.screensaver_config;
                match slot {
                    pages::EffectProfileSlot::Performance => cfg.effects_performance = text,
                    pages::EffectProfileSlot::Balanced => cfg.effects_balanced = text,
                    pages::EffectProfileSlot::Battery => cfg.effects_battery = text,
                    pages::EffectProfileSlot::Minimal => cfg.effects_minimal = text,
                }
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

    /// View for the Screensaver page
    pub(super) fn view_screensaver_page(&self) -> Element<'_, Message> {
        let spacing = cosmic::theme::spacing();
        let mut column = widget::column()
            .spacing(spacing.space_m)
            .padding(spacing.space_m)
            .push(widget::text::title2(fl!("screensaver")))
            .push(widget::text::body(fl!("screensaver-description")));

        let missing = ScreensaverConfig::missing_scripts();
        if !missing.is_empty() {
            let names = missing.join(", ");
            column = column.push(
                widget::container(widget::text::body(fl!(
                    "screensaver-scripts-missing",
                    names = names
                )))
                .padding(spacing.space_s)
                .class(cosmic::theme::Container::custom(|theme| {
                    let warn = theme.cosmic().warning_color();
                    widget::container::Style {
                        background: Some(cosmic::iced::Background::Color(
                            cosmic::iced::Color::from_rgba(warn.red, warn.green, warn.blue, 0.15),
                        )),
                        border: cosmic::iced::Border {
                            color: cosmic::iced::Color::from_rgba(
                                warn.red, warn.green, warn.blue, 0.5,
                            ),
                            width: 1.0,
                            radius: 4.0.into(),
                        },
                        ..widget::container::Style::default()
                    }
                })),
            );
        }

        column = column
            .push(self.view_screensaver_preview_section())
            .push(self.view_screensaver_settings_section());
        widget::scrollable(column).into()
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

    /// Power section: live D-Bus status + disable/idle config + per-profile effects
    #[allow(clippy::too_many_lines)]
    fn view_screensaver_power_section(&self) -> Element<'_, Message> {
        let cfg = &self.screensaver_config;

        let status_line = self.power_state.as_ref().map_or_else(
            || fl!("screensaver-power-status-unknown"),
            |p| {
                if p.on_battery {
                    p.battery_percent.map_or_else(
                        || fl!("screensaver-power-status-battery-no-pct"),
                        |pct| fl!("screensaver-power-status-battery", pct = pct),
                    )
                } else {
                    fl!("screensaver-power-status-ac")
                }
            },
        );

        let profile_line = self.power_state.as_ref().map(|p| {
            let label = match p.power_profile {
                crate::power::PowerProfile::Performance => "Performance",
                crate::power::PowerProfile::Balanced => "Balanced",
                crate::power::PowerProfile::PowerSaver => "Power Saver",
            };
            fl!("screensaver-power-profile", profile = label)
        });

        let s76_line = self.power_state.as_ref().map(|p| {
            if p.has_system76_power {
                fl!("screensaver-power-system76-yes")
            } else {
                fl!("screensaver-power-system76-no")
            }
        });

        let battery_timeout_options: Vec<String> = vec![
            fl!("screensaver-battery-timeout-5m"),
            fl!("screensaver-battery-timeout-10m"),
            fl!("screensaver-battery-timeout-15m"),
            fl!("screensaver-battery-timeout-30m"),
        ];
        let battery_timeout_values: [u32; 4] = [300, 600, 900, 1800];
        let battery_timeout_selected = battery_timeout_values
            .iter()
            .position(|&v| v == cfg.battery_idle_timeout);
        let battery_timeout_dropdown =
            widget::dropdown(battery_timeout_options, battery_timeout_selected, |index| {
                Message::Page(pages::Message::Screensaver(
                    pages::ScreensaverMessage::SetBatteryIdleTimeout(index),
                ))
            });

        let perf_input = widget::text_input(
            fl!("screensaver-effect-profile-placeholder"),
            &cfg.effects_performance,
        )
        .on_input(|text| {
            Message::Page(pages::Message::Screensaver(
                pages::ScreensaverMessage::SetEffectsForProfile(
                    pages::EffectProfileSlot::Performance,
                    text,
                ),
            ))
        });
        let bal_input = widget::text_input(
            fl!("screensaver-effect-profile-placeholder"),
            &cfg.effects_balanced,
        )
        .on_input(|text| {
            Message::Page(pages::Message::Screensaver(
                pages::ScreensaverMessage::SetEffectsForProfile(
                    pages::EffectProfileSlot::Balanced,
                    text,
                ),
            ))
        });
        let bat_input = widget::text_input(
            fl!("screensaver-effect-profile-placeholder"),
            &cfg.effects_battery,
        )
        .on_input(|text| {
            Message::Page(pages::Message::Screensaver(
                pages::ScreensaverMessage::SetEffectsForProfile(
                    pages::EffectProfileSlot::Battery,
                    text,
                ),
            ))
        });
        let min_input = widget::text_input(
            fl!("screensaver-effect-profile-placeholder"),
            &cfg.effects_minimal,
        )
        .on_input(|text| {
            Message::Page(pages::Message::Screensaver(
                pages::ScreensaverMessage::SetEffectsForProfile(
                    pages::EffectProfileSlot::Minimal,
                    text,
                ),
            ))
        });

        let mut section = widget::settings::section()
            .title(fl!("screensaver-power"))
            .add(widget::settings::item(
                fl!("screensaver-power-status"),
                widget::text::body(status_line),
            ));
        if let Some(line) = profile_line {
            section = section.add(widget::settings::item("", widget::text::body(line)));
        }
        if let Some(line) = s76_line {
            section = section.add(widget::settings::item("", widget::text::body(line)));
        }
        section = section
            .add(widget::settings::item(
                fl!("screensaver-disable-on-battery"),
                widget::toggler(cfg.disable_on_battery).on_toggle(|enabled| {
                    Message::Page(pages::Message::Screensaver(
                        pages::ScreensaverMessage::SetDisableOnBattery(enabled),
                    ))
                }),
            ))
            .add(widget::settings::item(
                fl!("screensaver-battery-idle-timeout"),
                battery_timeout_dropdown,
            ))
            .add(widget::settings::item(
                fl!("screensaver-effect-profile-performance"),
                perf_input,
            ))
            .add(widget::settings::item(
                fl!("screensaver-effect-profile-balanced"),
                bal_input,
            ))
            .add(widget::settings::item(
                fl!("screensaver-effect-profile-battery"),
                bat_input,
            ))
            .add(widget::settings::item(
                fl!("screensaver-effect-profile-minimal"),
                min_input,
            ));

        section.into()
    }

    /// Settings sub-section: status, clock, cursor & dismiss, session lock, save button
    #[allow(clippy::too_many_lines)]
    fn view_screensaver_settings_section(&self) -> Element<'_, Message> {
        let spacing = cosmic::theme::spacing();
        let cfg = &self.screensaver_config;

        // --- Terminal dropdown ---
        let terminal_options: Vec<String> = vec![
            fl!("screensaver-terminal-alacritty"),
            fl!("screensaver-terminal-ghostty"),
            fl!("screensaver-terminal-cosmic-term"),
        ];
        let terminal_values = ["alacritty", "ghostty", "cosmic-term"];
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

        let clock_font_input =
            widget::text_input(fl!("screensaver-clock-font-placeholder"), &cfg.clock_font)
                .on_input(|font| {
                    Message::Page(pages::Message::Screensaver(
                        pages::ScreensaverMessage::SetClockFont(font),
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
                    ))
                    .add(widget::settings::item(
                        fl!("screensaver-clock-font"),
                        clock_font_input,
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
            // Power section: live status + battery toggles + per-profile effects
            .push(self.view_screensaver_power_section())
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
