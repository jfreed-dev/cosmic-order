# cosmic-settings Research

Research findings on the cosmic-settings application architecture and patterns.

## Overview

**Repository**: <https://github.com/pop-os/cosmic-settings>
**License**: GPL-3.0-only
**Language**: Rust (99.6%)

COSMIC Settings is the official configuration application for the COSMIC desktop
environment. It demonstrates production-quality libcosmic patterns.

## Key Finding: No Plugin System

**Important**: COSMIC Settings does **NOT** have a runtime plugin system.

- All pages are compile-time selected via Cargo features
- Adding pages requires code changes and recompilation
- Third-party extensions are not supported

### Implications for COSMIC ORDER

Since we cannot extend cosmic-settings, we must build a **standalone application**
that:

1. Uses the same libcosmic toolkit
2. Integrates with the same configuration systems
3. Provides a native COSMIC experience
4. Can be installed alongside cosmic-settings

## Architecture

### Project Structure

```text
cosmic-settings/
├── src/
│   ├── main.rs          # Entry point, CLI subcommands
│   ├── app.rs           # Main SettingsApp, message routing
│   ├── config.rs        # Configuration via cosmic_config
│   ├── theme.rs         # Theme integration
│   ├── localize.rs      # i18n with Fluent
│   ├── pages/           # Settings page implementations
│   ├── subscription/    # Async event subscriptions
│   └── widget/          # Custom reusable widgets
├── page/                # Page trait definitions (separate crate)
└── crates/
    └── cosmic-pipewire/ # Audio integration
```

### Message Flow

```text
User Action
    ↓
Page Widget emits message
    ↓
Message::PageMessage(pages::Message::*Page*(variant))
    ↓
app.rs update() routes to correct page
    ↓
page.update() processes & updates state
    ↓
View re-renders
```

## Page System

### Page Trait

All settings pages implement `page::Page<Message>`:

```rust
pub trait Page<Message> {
    fn set_id(&mut self, id: Entity);
    fn info(&self) -> page::Info;
    fn content(&mut self, binder: &mut Binder<Message>) -> Vec<Content>;
    fn on_enter(&mut self, ...) -> Task<Message>;
    fn on_leave(&mut self, ...) -> Task<Message>;
    fn update(&mut self, message: Message, ...) -> Task<Message>;
    fn view(&self, ...) -> Element<Message>;
}
```

### Page Info

```rust
fn info(&self) -> page::Info {
    page::Info::new("page-id", "icon-name-symbolic")
        .title(fl!("page-title"))
        .description(fl!("page-description"))
}
```

### Sub-pages via AutoBind

```rust
impl AutoBind<Message> for Page {
    fn sub_pages(mut page: Binder<Message>) -> Binder<Message> {
        page = page.sub_page::<SubPage1>();
        page = page.sub_page::<SubPage2>();
        page
    }
}
```

### Section Organization

Pages use sections for logical grouping:

```rust
fn content(&mut self, binder: &mut Binder<Message>) -> Vec<Content> {
    vec![
        Content::Section(general_section(binder, self)),
        Content::Section(advanced_section(binder, self)),
    ]
}

fn general_section(binder: &Binder<M>, model: &Page) -> Section<M> {
    Section::default()
        .title(fl!("general"))
        .view(|binder, model, section| {
            widget::column()
                .push(/* widgets */)
                .into()
        })
}
```

## Configuration System

### cosmic_config Usage

```rust
use cosmic_config::{Config, ConfigGet, ConfigSet};

pub struct AppConfig {
    config: Config,
}

impl AppConfig {
    pub fn new() -> Result<Self, cosmic_config::Error> {
        let config = Config::new("com.example.app", 1)?;
        Ok(Self { config })
    }

    pub fn get_setting(&self) -> String {
        self.config.get("setting_key").unwrap_or_default()
    }

    pub fn set_setting(&self, value: &str) -> Result<(), cosmic_config::Error> {
        self.config.set("setting_key", value)
    }
}
```

### Configuration Locations

| Type | Path |
|------|------|
| User config | `~/.config/cosmic/<app-id>/v<version>/` |
| System defaults | `/usr/share/cosmic/<app-id>/v<version>/` |
| Theme (dark) | `~/.config/cosmic/com.system76.CosmicTheme.Dark.Builder/v1/` |
| Theme (light) | `~/.config/cosmic/com.system76.CosmicTheme.Light.Builder/v1/` |
| Background | `~/.config/cosmic/com.system76.CosmicBackground/v1/` |

### File Format

Configuration uses RON (Rusty Object Notation):

```ron
(
    setting_name: "value",
    numeric_setting: 42,
    color: (red: 0.5, green: 0.5, blue: 1.0, alpha: 1.0),
)
```

## Localization

### Fluent-based i18n

```rust
// Define loader
static LANGUAGE_LOADER: LanguageLoader = ...;

// Translation macro
macro_rules! fl {
    ($msg_id:expr) => { fl!($msg_id,) };
    ($msg_id:expr, $($key:expr => $value:expr),*) => {
        i18n_embed_fl::fl!(LANGUAGE_LOADER, $msg_id, $($key => $value),*)
    }
}

// Usage
let title = fl!("page-title");
let greeting = fl!("greeting", name => user_name);
```

### Translation Files

Located in `i18n/<lang>/`:

```fluent
# i18n/en/app.ftl
page-title = Settings
greeting = Hello, { $name }!
```

## Async Subscriptions

For real-time updates (Bluetooth, network, etc.):

```rust
pub fn subscription() -> Subscription<Event> {
    Subscription::run_with_id("subscription-id", async move {
        // Emit events
        yield Event::Loading;

        // Do async work
        let data = fetch_data().await;
        yield Event::Loaded(data);

        // Keep alive
        future::pending().await
    })
}
```

## Custom Widgets

Reusable widget patterns from cosmic-settings:

```rust
// Navigation item
pub fn go_next_item(label: impl Into<String>) -> Element<Message> {
    widget::row()
        .push(widget::text(label))
        .push(widget::icon::from_name("go-next-symbolic"))
        .into()
}

// Page header with back button
pub fn sub_page_header(back_label: impl Into<String>) -> Element<Message> {
    widget::row()
        .push(widget::button::icon("go-previous-symbolic"))
        .push(widget::text::title3(back_label))
        .into()
}
```

## Build System

### justfile Commands

```bash
just              # Build release
just run          # Run with logging
just check        # Lint with clippy
just install      # Install to system
just build-deb    # Build Debian package
```

### Feature Flags

```toml
[features]
default = ["linux", "page-accessibility", "page-applications", ...]

# Optional pages
page-accessibility = []
page-bluetooth = []
page-display = []
page-networking = []
page-power = []
page-sound = []

# Platform features
wayland = []
xdg-portal = []
```

## Patterns to Adopt

### 1. Page Structure

```rust
pub struct Page {
    id: Entity,
    // Page state
}

pub enum Message {
    // Page-specific messages
}

impl page::Page<crate::pages::Message> for Page {
    // Implement required methods
}
```

### 2. Error Handling

```rust
// Never panic
match result {
    Ok(value) => self.data = value,
    Err(e) => tracing::error!("Operation failed: {e}"),
}
```

### 3. Async Operations

```rust
fn update(&mut self, message: Message) -> Task<Message> {
    match message {
        Message::Save => {
            let data = self.data.clone();
            Task::perform(
                async move { save_data(data).await },
                |result| match result {
                    Ok(_) => Message::Saved,
                    Err(e) => Message::Error(e.to_string()),
                }
            )
        }
        _ => Task::none()
    }
}
```

### 4. Theme Integration

```rust
let spacing = cosmic::theme::spacing();
let theme = cosmic::theme::active();

widget::container(content)
    .padding(spacing.space_m)
    .style(cosmic::theme::Container::Card)
```

## Resources

- [cosmic-settings Repository](https://github.com/pop-os/cosmic-settings)
- [libcosmic Documentation](https://pop-os.github.io/libcosmic/cosmic/)
- [COSMIC Desktop](https://system76.com/cosmic)
