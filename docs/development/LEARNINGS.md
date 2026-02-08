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
- `palette` - Full color palette (gray_1, gray_2, neutral_0-10,
  bright colors)

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
- Structure: One directory per theme (catppuccin, gruvbox, nord,
  etc.)
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

Project uses strict clippy with `-D warnings`. Common allows
needed for placeholder code:

```rust
#[allow(dead_code)]           // Unused fields/functions for future use
#[allow(clippy::unused_self)] // Methods that will use self later
#[allow(clippy::missing_const_for_fn)] // Functions that won't stay const
```

### Rust 2024 Edition

Project uses `edition = "2024"` which requires Rust 1.85+. Some
rustfmt options are nightly-only and were commented out:

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

## 2026-01-28: Phase 3 — Wallpaper Management

### libcosmic Widget APIs

**`widget::flex_row`** — responsive wrapping grid layout:

```rust
widget::flex_row(cards)
    .column_spacing(spacing.space_s)
    .row_spacing(spacing.space_s)
    .width(Length::Fill)
```

Key methods: `.column_spacing()`, `.row_spacing()`, `.spacing()`
(both), `.width()`, `.min_item_width()`, `.align_items()`,
`.justify_content()`.

**`widget::button::image`** — image button with selection support:

```rust
widget::button::image(Handle::from_path(path))
    .width(Length::Fixed(160.0))
    .height(Length::Fixed(100.0))
    .selected(is_selected)
    .on_press(Message::Select(id))
```

Key methods: `.selected(bool)`, `.on_press()`, `.on_remove()`,
`.width()`, `.height()`.

**`widget::dropdown`** — popover select menu:

```rust
// Takes ownership of options via Into<Cow<[S]>>
// Closure must be 'static — clone data for closure if needed
let options_for_closure = options.clone();
widget::dropdown(options, selected_index, move |index| {
    Message::Selected(options_for_closure[index].clone())
})
```

### Performance: Image Grid Rendering

**Problem**: Loading full-resolution wallpapers (5120x2880,
~1.4 MB each) as `button::image` thumbnails causes the UI to
freeze. Even 24 images overwhelm the iced renderer.

**Root cause**: `Handle::from_path` loads the full image into GPU
memory. The visual display size (160x100) doesn't reduce the
decode/upload cost.

**Solution — thumbnail cache + pagination**:

1. **Thumbnail cache** (`ThumbnailCache` in `wallpaper_config.rs`):
   - Uses `image` crate to generate 160x100 thumbnails
   - Stored at `~/.cache/cosmic-order/thumbnails/`
   - Cache key: `{theme}__{filename}` (avoids collisions)
   - Failed thumbnails cached as empty marker files (0 bytes) to
     prevent retry on every `view()` frame
   - Falls back to original path on failure

2. **Pagination** (12 per page):
   - `flex_row` layout with large element counts is expensive even
     with small images
   - 12 thumbnails per page keeps rendering smooth
   - `<` / `>` nav buttons with page counter

3. **No "All" option**: removed aggregate view entirely — too many
   images regardless of caching

**Key lesson**: In iced's Elm architecture, `view()` runs on every
frame. Any I/O or computation in `view()` blocks the UI thread.
Thumbnail generation must be cached, and failure must be cached
too (otherwise corrupt files retry every frame).

### RON Serialization for COSMIC Background Config

Proper round-trip serialization using serde + ron:

```rust
#[derive(Serialize, Deserialize)]
pub struct CosmicBgEntry {
    pub output: String,
    pub source: BgSource,
    pub filter_by_theme: bool,
    pub rotation_frequency: u32,
    pub filter_method: FilterMethod,
    pub scaling_mode: ScalingMode,
    pub sampling_method: SamplingMethod,
}

// Parse with fallback
if let Ok(entry) = ron::from_str::<CosmicBgEntry>(content) {
    // use entry
} else {
    // fallback to manual line parsing
}
```

### Async Patterns for File Operations

Theme export/import established the pattern; wallpaper
apply/import follows it:

```rust
// In update handler:
cosmic::task::future(async move {
    let result = run_async_operation().await;
    Message::OperationComplete(result)
})

// File dialog via xdg-portal:
use cosmic::dialog::file_chooser;
let dialog = file_chooser::open::Dialog::new()
    .title("Title")
    .filter(file_chooser::FileFilter::new("Images")
        .glob("*.png").glob("*.jpg"));
match dialog.open_file().await {
    Ok(response) => { /* use response.url().to_file_path() */ }
    Err(file_chooser::Error::Cancelled) => { /* user cancelled */ }
    Err(e) => { /* real error */ }
}
```

### App ID Mismatch (PR-01)

`main.rs` defined `APP_ID = "com.github.jfreed-dev.CosmicOrder"`
but `config.rs` used a separate hardcoded
`"com.system76.CosmicOrder"`. Config state was split across two
namespaces. Fix: `config.rs` now uses `crate::APP_ID`.

### Dependencies Added

| Crate | Purpose |
|-------|---------|
| `image` | Thumbnail generation (160x100 from 5K wallpapers) |

### Files Changed in Phase 3

| File | Changes |
|------|---------|
| `src/wallpaper_config.rs` | RON structs, errors, save/set, thumbnails, scanning |
| `src/pages/mod.rs` | `WallpapersMessage` enum (11 variants + pagination) |
| `src/app.rs` | Wallpaper state, message handler, async helpers, grid view |
| `i18n/en/cosmic_order.ftl` | 11 new wallpaper i18n strings |
| `src/config.rs` | Unified app ID (PR-01) |
| `Cargo.toml` | Added `image` crate |

---

## Next Session TODO

- [x] ~~Investigate theme label display issue~~ — Resolved:
  stale incremental build; `RustEmbed` embeds `.ftl` files at
  compile time and cargo may not detect `.ftl` changes.
  Fix: `cargo clean` forces re-embed.
- [x] Update dependencies (`cargo update`) — libcosmic updated
  to `#3e78eb23`, now requires Rust 1.90+
- [x] Fix clippy `manual_div_ceil` warning — replaced manual
  ceiling division with `.div_ceil()`
- [x] Phase 4: Screensaver configuration — complete (Phase 4/4A/4B/4C)
- [x] Async thumbnail generation — complete (Phase 5)
- [ ] PR-02: Full theme palette application (deferred)
- [ ] Test wallpaper Apply flow end-to-end
- [ ] Test wallpaper Import flow
