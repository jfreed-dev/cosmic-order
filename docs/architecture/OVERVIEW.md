# Architecture Overview

This document describes the high-level architecture of COSMIC Tweaks.

## Design Principles

1. **Native COSMIC Experience** - Follow COSMIC design patterns and conventions
2. **Configuration Integration** - Use standard COSMIC configuration systems
3. **Modularity** - Loosely coupled pages that can be developed independently
4. **Documentation-First** - Document design decisions before implementation
5. **Accessibility** - Support keyboard navigation and screen readers

## Application Structure

```text
cosmic-tweaks/
├── src/
│   ├── main.rs              # Entry point
│   ├── app.rs               # Application state and message routing
│   ├── config.rs            # Application configuration
│   ├── pages/
│   │   ├── mod.rs           # Page registry and routing
│   │   ├── themes/          # Theme management page
│   │   ├── wallpapers/      # Wallpaper management page
│   │   └── screensaver/     # Screensaver configuration page
│   └── widgets/
│       └── mod.rs           # Shared custom widgets
├── resources/
│   ├── icons/               # Application icons
│   └── i18n/                # Translation files
└── tests/
    └── ...                  # Integration tests
```

## Component Architecture

```text
┌─────────────────────────────────────────────────────────────────┐
│                         COSMIC Tweaks                           │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  ┌──────────────────────────────────────────────────────────┐  │
│  │                    Application Shell                      │  │
│  │  ┌─────────────┐  ┌──────────────────────────────────┐   │  │
│  │  │  Navigation │  │         Content Area              │   │  │
│  │  │   Sidebar   │  │  ┌────────────────────────────┐  │   │  │
│  │  │             │  │  │      Active Page           │  │   │  │
│  │  │  • Themes   │  │  │                            │  │   │  │
│  │  │  • Walls    │  │  │  - Sections                │  │   │  │
│  │  │  • Screen   │  │  │  - Settings                │  │   │  │
│  │  │             │  │  │  - Actions                 │  │   │  │
│  │  └─────────────┘  │  └────────────────────────────┘  │   │  │
│  │                   └──────────────────────────────────┘   │  │
│  └──────────────────────────────────────────────────────────┘  │
│                                                                 │
├─────────────────────────────────────────────────────────────────┤
│                      Integration Layer                          │
│  ┌────────────┐  ┌────────────┐  ┌────────────┐  ┌──────────┐ │
│  │ cosmic-    │  │ cosmic-    │  │ cosmic-bg- │  │ swayidle │ │
│  │ config     │  │ theme      │  │ config     │  │ config   │ │
│  └────────────┘  └────────────┘  └────────────┘  └──────────┘ │
└─────────────────────────────────────────────────────────────────┘
```

## Message Flow

Following the Elm architecture:

```text
                    ┌─────────────────┐
                    │   Application   │
                    │     State       │
                    └────────┬────────┘
                             │
              ┌──────────────┴──────────────┐
              ▼                             │
        ┌──────────┐                        │
        │   view   │                        │
        │ function │                        │
        └────┬─────┘                        │
             │                              │
             ▼                              │
    ┌─────────────────┐                     │
    │  Element Tree   │                     │
    │  (UI Widgets)   │                     │
    └────────┬────────┘                     │
             │                              │
             │ User Interaction             │
             ▼                              │
    ┌─────────────────┐                     │
    │    Message      │                     │
    └────────┬────────┘                     │
             │                              │
             ▼                              │
    ┌─────────────────┐                     │
    │     update      │─────────────────────┘
    │    function     │
    └─────────────────┘
```

## Page System

### Page Trait

Each page implements a common interface:

```rust
pub trait Page {
    /// Page metadata (id, title, icon)
    fn info(&self) -> PageInfo;

    /// Called when page becomes active
    fn on_enter(&mut self) -> Task<Message>;

    /// Called when leaving page
    fn on_leave(&mut self) -> Task<Message>;

    /// Handle page-specific messages
    fn update(&mut self, message: PageMessage) -> Task<Message>;

    /// Render page content
    fn view(&self) -> Element<Message>;
}
```

### Page Registration

Pages are registered in `pages/mod.rs`:

```rust
pub enum PageId {
    Themes,
    Wallpapers,
    Screensaver,
}

pub fn create_page(id: PageId) -> Box<dyn Page> {
    match id {
        PageId::Themes => Box::new(themes::Page::new()),
        PageId::Wallpapers => Box::new(wallpapers::Page::new()),
        PageId::Screensaver => Box::new(screensaver::Page::new()),
    }
}
```

## Configuration Integration

### Application Config

COSMIC Tweaks uses `cosmic-config` for its own settings:

```text
~/.config/cosmic/com.example.CosmicTweaks/v1/
├── config           # Application preferences
└── state            # Window state, last page, etc.
```

### External Configs (Read/Write)

| Config | Crate | Purpose |
|--------|-------|---------|
| `com.system76.CosmicTheme.*` | cosmic-theme | Theme settings |
| `com.system76.CosmicBackground` | cosmic-bg-config | Wallpaper settings |
| `cosmic-screensaver` | custom | Screensaver settings |

### Configuration Watch

Subscribe to external config changes:

```rust
fn subscription(&self) -> Subscription<Message> {
    cosmic::config_subscription(
        "theme-watcher",
        |config: cosmic_theme::Config| Message::ThemeChanged(config)
    )
}
```

## Error Handling Strategy

### Principles

1. **Never panic** in production code
2. **Log errors** with context
3. **Show user feedback** for actionable errors
4. **Graceful degradation** when possible

### Implementation

```rust
// Configuration errors
match config.get("setting") {
    Ok(value) => self.setting = value,
    Err(e) => {
        tracing::warn!("Failed to load setting, using default: {e}");
        self.setting = default_value();
    }
}

// User-facing errors
match save_theme(theme) {
    Ok(_) => Task::none(),
    Err(e) => {
        self.error_message = Some(format!("Failed to save theme: {e}"));
        Task::none()
    }
}
```

## Async Operations

### Task Pattern

Long-running operations use `Task`:

```rust
fn update(&mut self, message: Message) -> Task<Message> {
    match message {
        Message::LoadWallpapers => {
            Task::perform(
                async { load_wallpapers().await },
                Message::WallpapersLoaded
            )
        }
        Message::WallpapersLoaded(result) => {
            match result {
                Ok(wallpapers) => self.wallpapers = wallpapers,
                Err(e) => tracing::error!("Failed to load wallpapers: {e}"),
            }
            Task::none()
        }
        _ => Task::none()
    }
}
```

### Subscription Pattern

For real-time updates:

```rust
fn subscription(&self) -> Subscription<Message> {
    Subscription::batch([
        self.theme_subscription(),
        self.wallpaper_subscription(),
    ])
}
```

## Testing Strategy

### Unit Tests

Test individual components:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_theme_parsing() {
        let theme = parse_theme(SAMPLE_THEME);
        assert!(theme.is_ok());
    }
}
```

### Integration Tests

Test page behavior:

```rust
#[test]
fn test_theme_page_update() {
    let mut page = ThemesPage::new();
    let task = page.update(Message::SelectTheme(theme_id));
    // Verify state changes
}
```

## Security Considerations

1. **File paths** - Validate and sanitize user-provided paths
2. **Configuration** - Use cosmic-config's safe serialization
3. **External commands** - Avoid shell injection when spawning processes
4. **Permissions** - Request only necessary file access

## Performance Guidelines

1. **Lazy loading** - Load data on demand, not at startup
2. **Caching** - Cache expensive computations (thumbnails, theme previews)
3. **Async I/O** - Never block the UI thread
4. **Efficient rendering** - Use `Lazy` widget for large lists
