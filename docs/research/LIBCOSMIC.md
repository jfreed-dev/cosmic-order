# libcosmic Research

Research findings on the libcosmic toolkit for COSMIC application development.

## Overview

**Repository**: <https://github.com/pop-os/libcosmic>
**Documentation**: <https://pop-os.github.io/libcosmic/cosmic/>
**Learning Guide**: <https://pop-os.github.io/libcosmic-book/>
**License**: MPL-2.0

libcosmic is a Rust-based GUI toolkit built on [iced](https://github.com/iced-rs/iced)
for creating applications for the COSMIC desktop environment.

## Technology Stack

| Component | Technology |
|-----------|------------|
| Language | Rust (99.1%) |
| GUI Framework | iced (Elm-inspired) |
| Architecture | Functional reactive (Elm pattern) |
| Rendering | Software (softbuffer) or GPU (wgpu) |
| Async Runtime | tokio or smol |

## Key Dependencies

```toml
[dependencies]
iced = "..."              # Core GUI framework
cosmic-theme = "..."      # Theming system
cosmic-config = "..."     # Configuration management
cosmic-panel-config = "..." # Panel integration (for applets)
taffy = "..."             # CSS Grid layout
palette = "..."           # Color management
serde = "..."             # Serialization
zbus = "..."              # D-Bus (Linux)
```

## Application Architecture

Applications follow the **Elm architecture**:

```text
State → View → Message → Update → State
```

### Basic Structure

```rust
use cosmic::prelude::*;

pub struct App {
    core: Core,
    // Application state fields
}

pub enum Message {
    // Application messages/events
}

impl cosmic::Application for App {
    type Message = Message;
    type Executor = cosmic::executor::Default;
    type Flags = ();

    fn new(core: Core, _flags: Self::Flags) -> (Self, Task<Message>) {
        let app = App { core };
        (app, Task::none())
    }

    fn title(&self) -> String {
        String::from("My App")
    }

    fn update(&mut self, message: Self::Message) -> Task<Message> {
        match message {
            // Handle messages
        }
        Task::none()
    }

    fn view(&self, id: window::Id) -> Element<Self::Message> {
        // Build UI tree
        widget::column()
            .push(widget::text("Hello COSMIC"))
            .into()
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    cosmic::app::run::<App>(())
}
```

## Widget System

libcosmic provides 45+ pre-built widgets:

### Layout Widgets

- `Column` / `Row` - Flex layouts
- `Grid` - CSS Grid layout
- `Container` - Generic container
- `Scrollable` - Scroll containers
- `PaneGrid` - Resizable panes

### Interactive Widgets

- `Button`, `TextButton`, `IconButton`
- `TextInput`, `TextEditor`
- `Checkbox`, `Radio`, `Toggler`
- `Slider`, `SpinButton`
- `ColorPicker`, `Calendar`
- `ComboBox`, `SegmentedControl`

### Display Widgets

- `Text` with typography hierarchy
- `Icon` - System icons
- `Image` - Images with SVG support
- `ProgressBar`
- `Card`, `Divider`

### Navigation Widgets

- `NavBar` - Navigation bar
- `TabBar` - Tab navigation
- `HeaderBar` - Window header
- `ContextDrawer` - Side drawer
- `Menu`, `ContextMenu`

### Typography

```rust
widget::text::title1(text)   // H1
widget::text::title2(text)   // H2
widget::text::title3(text)   // H3
widget::text::body(text)     // Body text
widget::text::caption(text)  // Small text
```

## Theming

### Theme Types

```rust
pub enum ThemeType {
    Dark,
    Light,
    HighContrastDark,
    HighContrastLight,
    Custom(Arc<CosmicTheme>),
    System { prefer_dark: Option<bool>, theme: Arc<CosmicTheme> },
}
```

### Theme Access

```rust
// Get current theme
let theme = cosmic::theme::active();

// Check theme type
if cosmic::theme::is_dark() { ... }

// Get spacing from theme
let spacing = cosmic::theme::spacing();
widget::column().spacing(spacing.space_xs)
```

### Dynamic Theme Updates

```rust
// Subscribe to theme changes
cosmic::config_subscription()
```

## Configuration System

Uses `cosmic-config` for persistent settings:

```rust
use cosmic_config::{Config, ConfigGet, ConfigSet};

// Get config handle
let config = Config::new("com.example.myapp", 1)?;

// Read value
let value: String = config.get("setting_key")?;

// Write value
config.set("setting_key", "value")?;
```

Configuration stored in: `~/.config/cosmic/<app-id>/v<version>/`

## Build Requirements

### System Dependencies (Pop!_OS/Ubuntu)

```bash
sudo apt install cargo cmake just libexpat1-dev libfontconfig-dev \
  libfreetype-dev libxkbcommon-dev pkg-config
```

### Rust Requirements

- Edition: 2024
- Minimum version: 1.85

### Cargo Features

```toml
[features]
default = ["dbus-config", "multi-window", "a11y"]

# Async runtime (choose one)
tokio = ["dep:tokio"]
smol = ["dep:smol"]

# Rendering
wgpu = ["iced/wgpu"]  # GPU acceleration

# Platform
wayland = ["iced/wayland"]
winit = ["iced/winit"]

# Desktop integration
desktop = ["dep:freedesktop-desktop-entry"]
single-instance = ["dep:zbus"]
xdg-portal = ["dep:ashpd"]
```

## Project Templates

Official templates for new projects:

- **Application**: <https://github.com/pop-os/cosmic-app-template>
- **Applet**: <https://github.com/pop-os/cosmic-applet-template>

```bash
# Create new app from template
cargo generate --git https://github.com/pop-os/cosmic-app-template
```

## Examples

The libcosmic repository includes 15+ examples:

| Example | Description |
|---------|-------------|
| application | Basic app template |
| applet | Panel applet |
| calendar | Calendar widget |
| config | Configuration system |
| context-menu | Context menus |
| menu | Application menus |
| multi-window | Multiple windows |
| nav-context | Navigation + drawer |

Run examples:

```bash
just run <example-name>
```

## Best Practices

### Error Handling

Never use `.unwrap()` or `.expect()` in production code:

```rust
// Bad
let value = config.get("key").unwrap();

// Good
match config.get("key") {
    Ok(value) => { /* use value */ }
    Err(e) => tracing::error!("Failed to get config: {e}"),
}
```

### Theming

Always use theme values for spacing and colors:

```rust
let spacing = cosmic::theme::spacing();
widget::column()
    .spacing(spacing.space_s)
    .padding(spacing.space_m)
```

### Async Operations

Return `Task` for async operations:

```rust
fn update(&mut self, message: Message) -> Task<Message> {
    match message {
        Message::LoadData => {
            Task::perform(
                async { load_data().await },
                Message::DataLoaded
            )
        }
        Message::DataLoaded(data) => {
            self.data = data;
            Task::none()
        }
    }
}
```

## Resources

- [API Documentation](https://pop-os.github.io/libcosmic/cosmic/)
- [libcosmic Book](https://pop-os.github.io/libcosmic-book/)
- [GitHub Repository](https://github.com/pop-os/libcosmic)
- [iced Documentation](https://docs.rs/iced)
