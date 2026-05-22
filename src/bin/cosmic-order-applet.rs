// SPDX-License-Identifier: GPL-3.0-only

//! COSMIC panel applet for COSMIC ORDER.
//!
//! A small panel button that opens a popup of quick screensaver controls:
//! lock now, start the screensaver, toggle the screensaver service, and
//! open the main COSMIC ORDER settings. The applet holds no control logic
//! of its own — every action shells out to the same surface the GUI/CLI
//! already use (`loginctl`, the bundled screensaver scripts, and the
//! `cosmic-order` binary).

use cosmic::Element;
use cosmic::app::{Core, Task};
use cosmic::iced::window::Id;
use cosmic::iced::{Length, Rectangle};
use cosmic::iced_runtime::core::window;
use cosmic::surface::action::{app_popup, destroy_popup};
use cosmic::widget;
use std::path::{Path, PathBuf};
use std::process::Command;

// Reuse the main crate's localization (fl! macro + embedded i18n/) without a
// separate library target.
#[path = "../localize.rs"]
mod localize;

/// Wayland app id for the applet; must match the installed `.desktop` file.
const APP_ID: &str = "com.github.jfreed-dev.CosmicOrderApplet";

/// Systemd user unit toggled by the screensaver service.
const IDLE_SERVICE: &str = "cosmic-screensaver-idle.service";

struct Applet {
    core: Core,
    popup: Option<Id>,
    screensaver_enabled: bool,
}

#[derive(Clone, Debug)]
enum Message {
    PopupClosed(Id),
    Surface(cosmic::surface::Action),
    LockNow,
    StartScreensaver,
    ToggleScreensaver(bool),
    OpenSettings,
}

impl cosmic::Application for Applet {
    type Executor = cosmic::SingleThreadExecutor;
    type Flags = ();
    type Message = Message;
    const APP_ID: &'static str = APP_ID;

    fn core(&self) -> &Core {
        &self.core
    }

    fn core_mut(&mut self) -> &mut Core {
        &mut self.core
    }

    fn init(core: Core, _flags: Self::Flags) -> (Self, Task<Message>) {
        let applet = Applet {
            core,
            popup: None,
            screensaver_enabled: screensaver_active(),
        };
        (applet, Task::none())
    }

    fn on_close_requested(&self, id: window::Id) -> Option<Message> {
        Some(Message::PopupClosed(id))
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::PopupClosed(id) => {
                if self.popup.as_ref() == Some(&id) {
                    self.popup = None;
                }
            }
            Message::Surface(action) => {
                return cosmic::task::message(cosmic::Action::Cosmic(
                    cosmic::app::Action::Surface(action),
                ));
            }
            Message::LockNow => spawn("loginctl", &["lock-session"]),
            Message::StartScreensaver => {
                spawn_path(&screensaver_script("launch-fullscreen.sh"), &["launch"]);
            }
            Message::ToggleScreensaver(on) => {
                let action = if on { "enable" } else { "disable" };
                spawn_path(&screensaver_script("screensaver-ctl.sh"), &[action]);
                self.screensaver_enabled = on;
            }
            Message::OpenSettings => spawn("cosmic-order", &["--page", "screensaver"]),
        }
        Task::none()
    }

    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    fn view(&self) -> Element<'_, Message> {
        let title = fl!("app-title");
        let have_popup = self.popup;

        let button = self
            .core
            .applet
            .icon_button("com.github.jfreed-dev.CosmicOrder")
            .on_press_with_rectangle(move |offset, bounds| {
                if let Some(id) = have_popup {
                    Message::Surface(destroy_popup(id))
                } else {
                    Message::Surface(app_popup::<Self>(
                        move |state: &mut Self| {
                            let new_id = Id::unique();
                            state.popup = Some(new_id);
                            // Reflect the live service state each time we open.
                            state.screensaver_enabled = screensaver_active();

                            let parent = state.core.main_window_id().unwrap_or(Id::NONE);
                            let mut settings = state
                                .core
                                .applet
                                .get_popup_settings(parent, new_id, None, None, None);
                            settings.positioner.anchor_rect = Rectangle {
                                x: (bounds.x - offset.x) as i32,
                                y: (bounds.y - offset.y) as i32,
                                width: bounds.width as i32,
                                height: bounds.height as i32,
                            };
                            settings
                        },
                        Some(Box::new(move |state: &Self| {
                            let content = widget::list_column()
                                .padding([8, 0])
                                .spacing(0)
                                .add(action_row(fl!("applet-lock-now"), Message::LockNow))
                                .add(action_row(
                                    fl!("applet-start-screensaver"),
                                    Message::StartScreensaver,
                                ))
                                .add(widget::settings::item(
                                    fl!("applet-screensaver-enabled"),
                                    widget::toggler(state.screensaver_enabled)
                                        .on_toggle(Message::ToggleScreensaver),
                                ))
                                .add(widget::divider::horizontal::default())
                                .add(action_row(
                                    fl!("applet-open-settings"),
                                    Message::OpenSettings,
                                ));

                            Element::from(state.core.applet.popup_container(content))
                                .map(cosmic::Action::App)
                        })),
                    ))
                }
            });

        Element::from(self.core.applet.applet_tooltip::<Message>(
            button,
            title,
            self.popup.is_some(),
            Message::Surface,
            None,
        ))
    }

    fn view_window(&self, _id: Id) -> Element<'_, Message> {
        widget::text("").into()
    }

    fn style(&self) -> Option<cosmic::iced_core::theme::Style> {
        Some(cosmic::applet::style())
    }
}

/// A full-width text button used as a popup menu row.
fn action_row(label: String, message: Message) -> Element<'static, Message> {
    widget::button::text(label)
        .width(Length::Fill)
        .on_press(message)
        .into()
}

/// Whether the screensaver idle service is currently active.
fn screensaver_active() -> bool {
    Command::new("systemctl")
        .args(["--user", "is-active", "--quiet", IDLE_SERVICE])
        .status()
        .is_ok_and(|status| status.success())
}

/// Resolve a bundled screensaver script, preferring an env override and the
/// standard install location.
fn screensaver_script(name: &str) -> PathBuf {
    let candidates = [
        std::env::var("COSMIC_ORDER_SCREENSAVER_DIR").unwrap_or_default(),
        "/usr/share/cosmic-order/screensaver".to_owned(),
        "/usr/local/share/cosmic-order/screensaver".to_owned(),
    ];
    for dir in candidates.iter().filter(|dir| !dir.is_empty()) {
        let path = Path::new(dir).join(name);
        if path.exists() {
            return path;
        }
    }
    Path::new("/usr/share/cosmic-order/screensaver").join(name)
}

/// Spawn a command by name, logging (but not propagating) failures.
fn spawn(program: &str, args: &[&str]) {
    if let Err(err) = Command::new(program).args(args).spawn() {
        eprintln!("cosmic-order-applet: failed to run {program}: {err}");
    }
}

/// Spawn an executable by path, logging (but not propagating) failures.
fn spawn_path(path: &Path, args: &[&str]) {
    if let Err(err) = Command::new(path).args(args).spawn() {
        eprintln!(
            "cosmic-order-applet: failed to run {}: {err}",
            path.display()
        );
    }
}

fn main() -> cosmic::iced::Result {
    let requested = i18n_embed::DesktopLanguageRequester::requested_languages();
    localize::init(&requested);
    cosmic::applet::run::<Applet>(())
}
