# Development Learnings

Notes and discoveries from development sessions. Reference for future work.

---

## 2026-01-24: Config Reading & Display

### COSMIC Theme API

The current theme is accessible via libcosmic:

```rust
use cosmic::cosmic_theme::palette::Srgba;
use cosmic::theme;

let active_theme = theme::active();
let cosmic = active_theme.cosmic();

// Available fields:
cosmic.name          // String: "cosmic-dark"
cosmic.is_dark       // bool
cosmic.accent.base   // Srgba color
cosmic.background.base
cosmic.primary.on    // Text color
```

### COSMIC Config File Locations

| Config | Path | Format |
|--------|------|--------|
| Theme (Dark) | `~/.config/cosmic/com.system76.CosmicTheme.Dark/v1/` | RON files |
| Theme (Light) | `~/.config/cosmic/com.system76.CosmicTheme.Light/v1/` | RON files |
| Background | `~/.config/cosmic/com.system76.CosmicBackground/v1/all` | RON |
| Screensaver | `~/.config/cosmic-screensaver/config` | Shell KEY="value" |

### Theme Config Files

Each theme directory contains individual RON files:

- `accent` - Accent color with base/hover/pressed/selected states
- `background` - Background colors
- `name` - Theme name string
- `is_dark` - Boolean
- `palette` - Full color palette (gray_1, gray_2, neutral_0-10, bright colors)

### Background Config Format (RON)

```ron
(
    output: "all",
    source: Path("/usr/share/backgrounds/theme/wallpaper.png"),
    filter_by_theme: true,
    rotation_frequency: 600,  // seconds
    filter_method: Lanczos,
    scaling_mode: Zoom,       // Zoom, Fit, Stretch, etc.
    sampling_method: Random,
)
```

### Screensaver Config Format (Shell)

```bash
ENABLED="true"
IDLE_TIMEOUT="300"
LOCK_TIMEOUT="600"
DPMS_TIMEOUT="900"
FRAME_RATE="60"
INCLUDE_EFFECTS=""
EXCLUDE_EFFECTS="dev_worm"
SHOW_CLOCK="false"
LOGO_FILE="/path/to/logo.txt"
TERMINAL="ghostty"
```

### System Wallpaper Location

- Path: `/usr/share/backgrounds/`
- Structure: One directory per theme (catppuccin, gruvbox, nord, etc.)
- Current system: 15 themes, 299 total wallpapers

### libcosmic UI Patterns

**Settings sections with items:**

```rust
widget::settings::section()
    .title("Section Title")
    .add(widget::settings::item(
        "Label",
        widget::text::body("Value"),
    ))
```

**Page layout:**

```rust
widget::column()
    .spacing(spacing.space_m)
    .padding(spacing.space_m)
    .push(widget::text::title2(fl!("page-title")))
    .push(widget::text::body(fl!("description")))
    .push(settings_section)
    .into()
```

### Clippy Strictness

Project uses strict clippy with `-D warnings`. Common allows needed for placeholder code:

```rust
#[allow(dead_code)]           // Unused fields/functions for future use
#[allow(clippy::unused_self)] // Methods that will use self later
#[allow(clippy::missing_const_for_fn)] // Functions that won't stay const
```

### Rust 2024 Edition

Project uses `edition = "2024"` which requires Rust 1.85+. Some rustfmt options
are nightly-only and were commented out:

- `imports_granularity`
- `group_imports`
- `normalize_comments`
- `wrap_comments`

### Dependencies Used

| Crate | Purpose |
|-------|---------|
| `libcosmic` | COSMIC UI framework |
| `cosmic-config` | COSMIC configuration system |
| `directories` | Cross-platform config paths (`BaseDirs::new()`) |
| `ron` | RON format parsing (available but not yet used directly) |
| `thiserror` | Error type derivation |

---

## Next Session TODO

- [ ] Add interactive controls (toggles for enable/disable)
- [ ] Implement cosmic-config integration for app preferences
- [ ] Consider adding color swatches (visual color display)
- [ ] Wallpaper thumbnails (requires image loading)
- [ ] Test screensaver button (spawn screensaver-ctl.sh test)
