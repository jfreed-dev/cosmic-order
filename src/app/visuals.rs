// SPDX-License-Identifier: GPL-3.0-only

#[allow(clippy::wildcard_imports)]
use super::*;

impl App {
    /// Handle theme page messages
    #[allow(clippy::cognitive_complexity, clippy::too_many_lines)]
    pub(super) fn handle_themes_message(&mut self, message: pages::ThemesMessage) -> Task<Message> {
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
                Self::save_tool_sync_config(&self.tool_sync_config)
            }
            pages::ThemesMessage::SetBtopSync(enabled) => {
                self.tool_sync_config.btop_enabled = enabled;
                Self::save_tool_sync_config(&self.tool_sync_config)
            }
            pages::ThemesMessage::SetNvimSync(enabled) => {
                self.tool_sync_config.nvim_enabled = enabled;
                Self::save_tool_sync_config(&self.tool_sync_config)
            }
            pages::ThemesMessage::SetZellijSync(enabled) => {
                self.tool_sync_config.zellij_enabled = enabled;
                Self::save_tool_sync_config(&self.tool_sync_config)
            }
            pages::ThemesMessage::SetFzfSync(enabled) => {
                self.tool_sync_config.fzf_enabled = enabled;
                Self::save_tool_sync_config(&self.tool_sync_config)
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
                Self::save_tool_sync_config(&self.tool_sync_config)
            }
            pages::ThemesMessage::SetHooksEnabled(enabled) => {
                self.tool_sync_config.hooks_enabled = enabled;
                Self::save_tool_sync_config(&self.tool_sync_config)
            }
            pages::ThemesMessage::SetAutoSync(enabled) => {
                self.tool_sync_config.auto_sync = enabled;
                Self::save_tool_sync_config(&self.tool_sync_config)
            }
            pages::ThemesMessage::SyncTools => {
                self.tool_sync_status = None;
                let config = self.tool_sync_config.clone();
                cosmic::task::future(async move {
                    let msg = match crate::tool_sync::sync_tools(&config).await {
                        Ok(r) => {
                            let live = crate::tool_sync::signal_running_apps(&config);
                            let mut summary = r.summary();
                            if !live.reloaded.is_empty() {
                                use std::fmt::Write;
                                let _ = write!(summary, ", live: {}", live.reloaded.join(", "));
                            }
                            if !live.skipped.is_empty() {
                                use std::fmt::Write;
                                let _ = write!(summary, ", manual: {}", live.skipped.join(", "));
                            }
                            Ok(summary)
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
    pub(super) fn view_visuals_page(&self) -> Element<'_, Message> {
        // Show wizard view when active
        if self.wizard_state.is_some() {
            return self.view_wizard();
        }

        let spacing = cosmic::theme::spacing();

        let mut column = widget::column()
            .spacing(spacing.space_m)
            .padding(spacing.space_m)
            .push(widget::text::title2(fl!("visuals")));

        // Insert preview banner when actively previewing
        if let Some(banner) = self.view_preview_banner() {
            column = column.push(banner);
        }

        column = column
            // Community theme selectors (left) + theme preview (right)
            .push(
                widget::row()
                    .spacing(spacing.space_m)
                    .push(
                        widget::container(self.view_theme_selectors())
                            .width(cosmic::iced::Length::FillPortion(2)),
                    )
                    .push(
                        widget::container(self.view_theme_preview_panel())
                            .width(cosmic::iced::Length::FillPortion(1)),
                    ),
            )
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
            widget::container(
                widget::Space::new()
                    .width(cosmic::iced::Length::Fixed(22.0))
                    .height(cosmic::iced::Length::Fixed(22.0)),
            )
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
            fl!("wizard-light-mode")
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
                    widget::container(
                        widget::Space::new()
                            .width(Length::Fill)
                            .height(Length::Fixed(8.0)),
                    )
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
                .push(
                    widget::container(
                        widget::Space::new()
                            .width(Length::Fixed(130.0))
                            .height(Length::Fixed(5.0)),
                    )
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
                    widget::container(
                        widget::Space::new()
                            .width(Length::Fixed(90.0))
                            .height(Length::Fixed(5.0)),
                    )
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
                    widget::container(
                        widget::Space::new()
                            .width(Length::Fixed(110.0))
                            .height(Length::Fixed(5.0)),
                    )
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
                    widget::container(
                        widget::Space::new()
                            .width(Length::Fixed(50.0))
                            .height(Length::Fixed(10.0)),
                    )
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
        .width(Length::Fill)
        .max_width(300.0)
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

        let builtin_previews = crate::theme_config::ThemePreview::built_in_themes();
        let builtin_names: Vec<String> = builtin_previews.iter().map(|p| p.name.clone()).collect();
        let builtin_selected =
            previewing_id.and_then(|id| builtin_previews.iter().position(|p| p.id == id));
        let builtin_ids: Vec<crate::theme_config::ThemeId> =
            builtin_previews.iter().map(|p| p.id).collect();
        let builtin_dropdown = widget::dropdown(builtin_names, builtin_selected, move |idx| {
            let theme_id = builtin_ids[idx];
            Message::Page(pages::Message::Visuals(pages::ThemesMessage::PreviewTheme(
                theme_id,
            )))
        });

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

        widget::column()
            .spacing(cosmic::theme::spacing().space_s)
            .push(
                widget::settings::section()
                    .title(fl!("builtin-themes"))
                    .add(widget::settings::item(
                        fl!("builtin-themes-select"),
                        builtin_dropdown,
                    )),
            )
            .push(
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
                    )),
            )
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

    /// Save tool sync config and return a no-op `SyncComplete` message
    fn save_tool_sync_config(config: &crate::tool_sync::ToolSyncConfig) -> Task<Message> {
        let config = config.clone();
        cosmic::task::future(async move {
            if let Err(e) = config.save().await {
                tracing::warn!("Failed to save tool sync config: {e}");
            }
            Message::Page(pages::Message::Visuals(pages::ThemesMessage::SyncComplete(
                Ok(String::new()),
            )))
        })
    }
}
