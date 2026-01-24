// SPDX-License-Identifier: GPL-3.0-only

//! Main application module
//!
//! Implements the COSMIC Application trait and handles message routing.

use cosmic::app::{Core, Task};
use cosmic::iced::window;
use cosmic::widget::{self, nav_bar};
use cosmic::{Application, Element};

use crate::config::Config;
use crate::localize::fl;
use crate::pages::{self, PageId};

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
}

/// Application messages
#[derive(Debug, Clone)]
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

        // Build navigation model
        let mut nav_model = nav_bar::Model::default();

        // Add navigation items
        nav_model.insert()
            .text(fl!("themes"))
            .icon(widget::icon::from_name("preferences-desktop-theme-symbolic"))
            .data(PageId::Themes);

        nav_model.insert()
            .text(fl!("wallpapers"))
            .icon(widget::icon::from_name("preferences-desktop-wallpaper-symbolic"))
            .data(PageId::Wallpapers);

        nav_model.insert()
            .text(fl!("screensaver"))
            .icon(widget::icon::from_name("preferences-desktop-screensaver-symbolic"))
            .data(PageId::Screensaver);

        // Activate first item
        nav_model.activate_position(0);

        let app = App {
            core,
            config,
            nav_model,
            active_page: PageId::Themes,
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
        }
        Task::none()
    }

    fn update(&mut self, message: Self::Message) -> Task<Message> {
        match message {
            Message::NavSelect(id) => self.on_nav_select(id),
            Message::Page(page_message) => {
                // Route to appropriate page
                // TODO: Implement page message routing
                tracing::debug!("Page message: {:?}", page_message);
                Task::none()
            }
            Message::ConfigChanged(config) => {
                self.config = config;
                Task::none()
            }
        }
    }

    fn view(&self) -> Element<Self::Message> {
        // Build page content based on active page
        let content = match self.active_page {
            PageId::Themes => self.view_themes_page(),
            PageId::Wallpapers => self.view_wallpapers_page(),
            PageId::Screensaver => self.view_screensaver_page(),
        };

        content
    }

    fn header_start(&self) -> Vec<Element<Self::Message>> {
        vec![]
    }

    fn header_center(&self) -> Vec<Element<Self::Message>> {
        vec![widget::text::title3(fl!("app-title")).into()]
    }

    fn header_end(&self) -> Vec<Element<Self::Message>> {
        vec![]
    }
}

impl App {
    /// View for the Themes page
    fn view_themes_page(&self) -> Element<Message> {
        let spacing = cosmic::theme::spacing();

        widget::column()
            .spacing(spacing.space_m)
            .padding(spacing.space_m)
            .push(widget::text::title2(fl!("themes")))
            .push(widget::text::body(fl!("themes-description")))
            .push(
                widget::container(
                    widget::text::body("Theme management coming in Phase 2")
                )
                .padding(spacing.space_l)
                .style(cosmic::theme::Container::Card)
            )
            .into()
    }

    /// View for the Wallpapers page
    fn view_wallpapers_page(&self) -> Element<Message> {
        let spacing = cosmic::theme::spacing();

        widget::column()
            .spacing(spacing.space_m)
            .padding(spacing.space_m)
            .push(widget::text::title2(fl!("wallpapers")))
            .push(widget::text::body(fl!("wallpapers-description")))
            .push(
                widget::container(
                    widget::text::body("Wallpaper management coming in Phase 3")
                )
                .padding(spacing.space_l)
                .style(cosmic::theme::Container::Card)
            )
            .into()
    }

    /// View for the Screensaver page
    fn view_screensaver_page(&self) -> Element<Message> {
        let spacing = cosmic::theme::spacing();

        widget::column()
            .spacing(spacing.space_m)
            .padding(spacing.space_m)
            .push(widget::text::title2(fl!("screensaver")))
            .push(widget::text::body(fl!("screensaver-description")))
            .push(
                widget::container(
                    widget::text::body("Screensaver configuration coming in Phase 4")
                )
                .padding(spacing.space_l)
                .style(cosmic::theme::Container::Card)
            )
            .into()
    }
}
