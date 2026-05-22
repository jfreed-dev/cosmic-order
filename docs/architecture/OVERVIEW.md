# Architecture Overview

This document describes the high-level architecture of COSMIC ORDER.

## Design Principles

1. **Native COSMIC Experience** - Follow COSMIC design patterns and conventions
2. **Configuration Integration** - Use standard COSMIC configuration systems
3. **Modularity** - Loosely coupled pages that can be developed independently
4. **Documentation-First** - Document design decisions before implementation
5. **Accessibility** - Support keyboard navigation and screen readers

## Application Structure

```text
cosmic-order/
├── src/
│   ├── main.rs               # Entry point, APP_ID
│   ├── app/
│   │   ├── mod.rs            # Application state, Message enum, routing
│   │   ├── visuals.rs        # Visuals page: themes, wizard, preview
│   │   ├── screensaver.rs    # Screensaver page: views + handlers
│   │   ├── tool_sync_view.rs # Tool Sync page view
│   │   └── idle.rs           # Idle/sleep/lock event handlers
│   ├── config.rs             # Application configuration (cosmic-config)
│   ├── localize.rs           # i18n support (fl! macro)
│   ├── pages/
│   │   └── mod.rs            # PageId enum, Message routing, per-page message enums
│   ├── paths.rs              # Centralized config path resolution
│   ├── colors.rs             # ColorPalette, hex/RGB conversion utilities
│   ├── theme_config.rs       # Theme reading/writing, ThemePreview, export/import
│   ├── screensaver_config.rs # Screensaver config parsing
│   ├── generators/
│   │   ├── mod.rs            # Generator module declarations
│   │   ├── ghostty.rs        # Ghostty theme generator
│   │   ├── btop.rs           # btop theme generator
│   │   ├── nvim.rs           # Neovim theme generator
│   │   ├── zellij.rs         # Zellij theme generator
│   │   ├── fzf.rs            # fzf theme generator
│   │   └── lazygit.rs        # lazygit theme generator
│   ├── hooks.rs              # Custom hook execution
│   ├── tool_sync.rs          # Tool sync orchestration and per-tool config
│   ├── compositor.rs         # COSMIC compositor settings (cosmic-config API)
│   ├── cosmic_idle.rs        # DPMS timeout sync with cosmic-idle
│   ├── wayland_idle.rs       # Native Wayland idle detection
│   ├── sleep_lock.rs         # Sleep lock via logind D-Bus
│   ├── power.rs              # Power monitoring (UPower D-Bus subscription)
│   └── systemd.rs            # Systemd D-Bus unit restart
├── i18n/
│   └── en/
│       └── cosmic_order.ftl  # English translations
└── docs/
    ├── ROADMAP.md             # Development phases
    ├── architecture/          # System design
    └── development/           # Dev guides
```

## Component Architecture

```text
┌─────────────────────────────────────────────────────────────────┐
│                         COSMIC ORDER                           │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  ┌──────────────────────────────────────────────────────────┐  │
│  │                    Application Shell                      │  │
│  │  ┌─────────────┐  ┌──────────────────────────────────┐   │  │
│  │  │  Navigation │  │         Content Area              │   │  │
│  │  │   Sidebar   │  │  ┌────────────────────────────┐  │   │  │
│  │  │             │  │  │      Active Page           │  │   │  │
│  │  │  • Visuals  │  │  │                            │  │   │  │
│  │  │  • Tools    │  │  │  - Sections                │  │   │  │
│  │  │  • Screen   │  │  │  - Settings                │  │   │  │
│  │  │             │  │  │  - Actions                 │  │   │  │
│  │  └─────────────┘  │  └────────────────────────────┘  │   │  │
│  │                   └──────────────────────────────────┘   │  │
│  └──────────────────────────────────────────────────────────┘  │
│                                                                 │
├─────────────────────────────────────────────────────────────────┤
│                      Integration Layer                          │
│  ┌────────────┐  ┌────────────┐  ┌────────────┐  ┌──────────┐ │
│  │ cosmic-    │  │ cosmic-    │  │ swayidle   │  │ Wayland  │ │
│  │ config     │  │ theme      │  │ config     │  │ idle     │ │
│  └────────────┘  └────────────┘  └────────────┘  └──────────┘ │
├─────────────────────────────────────────────────────────────────┤
│                    Tool Sync Layer                               │
│  ┌────────────┐  ┌────────────┐  ┌────────────┐  ┌──────────┐ │
│  │ colors     │  │ generators │  │ D-Bus      │  │ logind   │ │
│  │ .toml      │  │ (6 tools)  │  │ (UPower)   │  │ lock     │ │
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

Pages are routed through the `App` struct's `view()` and `update()` methods.
Each page has its own message enum and dedicated handler/view methods in a
separate submodule of `src/app/`.

### Page Registration

Pages are declared in `pages/mod.rs`:

```rust
pub enum PageId {
    Visuals,
    ToolSync,
    Screensaver,
}

pub enum Message {
    Visuals(ThemesMessage),
    Screensaver(ScreensaverMessage),
}
```

### Page Modules

| Page | Message Handler | View | Module |
|------|----------------|------|--------|
| Visuals | `handle_themes_message` | `view_visuals_page` | `app/visuals.rs` |
| Tool Sync | (shares `ThemesMessage`) | `view_tool_sync_page` | `app/tool_sync_view.rs` |
| Screensaver | `handle_screensaver_message` | `view_screensaver_page` | `app/screensaver.rs` |

## Configuration Integration

### Application Config

COSMIC ORDER uses `cosmic-config` for its own settings:

```text
~/.config/cosmic/com.github.jfreed-dev.CosmicOrder/v1/
├── config           # Application preferences
└── state            # Window state, last page, etc.
```

### External Configs (Read/Write)

| Config | Crate | Purpose |
|--------|-------|---------|
| `com.system76.CosmicTheme.*` | cosmic-theme | Theme settings |
| `cosmic-screensaver` | custom | Screensaver settings |

### Theme Change Detection

Theme changes are detected via `Application` trait callbacks:

```rust
fn system_theme_update(&mut self, ...) -> Task<Message> {
    if self.tool_sync_config.auto_sync {
        self.update(Message::Page(pages::Message::Visuals(
            pages::ThemesMessage::SyncTools,
        )))
    } else { Task::none() }
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

Long-running operations use `cosmic::task::future`:

```rust
fn update(&mut self, message: Message) -> Task<Message> {
    match message {
        Message::SyncTools => {
            let config = self.tool_sync_config.clone();
            cosmic::task::future(async move {
                let result = tool_sync::sync_tools(&config).await;
                Message::SyncComplete(result)
            })
        }
        _ => Task::none()
    }
}
```

### Subscription Pattern

For real-time updates (D-Bus, Wayland idle):

```rust
fn subscription(&self) -> Subscription<Message> {
    Subscription::batch([
        power::power_subscription().map(Message::PowerStateUpdate),
        wayland_idle::idle_subscription(self.idle_config.clone())
            .map(Message::IdleEvent),
        sleep_lock::sleep_lock_subscription().map(Message::SleepEvent),
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
