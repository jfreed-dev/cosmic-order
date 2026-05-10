// SPDX-License-Identifier: GPL-3.0-only

#[allow(clippy::wildcard_imports)]
use super::*;

impl App {
    /// View for the Tool Sync page
    pub(super) fn view_tool_sync_page(&self) -> Element<'_, Message> {
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
}
