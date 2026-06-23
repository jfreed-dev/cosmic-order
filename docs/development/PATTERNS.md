# Implementation Patterns & Gotchas

Practical libcosmic/iced patterns and hard-won lessons used in COSMIC ORDER.
Complements the toolkit overview in
[`docs/research/LIBCOSMIC.md`](../research/LIBCOSMIC.md) (what the widgets are)
and the file map in
[`docs/architecture/OVERVIEW.md`](../architecture/OVERVIEW.md) (where things live).

---

## Performance: never do work in `view()`

libcosmic is built on iced's Elm architecture, where **`view()` is called on
every frame**. Any I/O, decoding, or heavy computation placed there blocks the
UI thread and causes visible stutter or freezes.

Rules that follow from this:

- **Do expensive work in `update()` / async tasks, not `view()`.** Compute
  once, store the result in state, and have `view()` only read it. Run anything
  slow off-thread with `cosmic::task::future` (see [Async work](#async-work-off-the-ui-thread)).
- **Cache expensive results** — and **cache failures too.** If a computation can
  fail on bad input (a corrupt file, a missing resource), record the failure
  (e.g. an empty marker) so `view()` doesn't retry it every single frame. An
  uncached failure in `view()` becomes a per-frame retry storm.
- **GPU image cost is decode + upload, not display size.** Loading a
  full-resolution image as a small thumbnail does **not** save memory or time —
  the renderer still decodes and uploads the full image. Pre-generate
  appropriately sized assets and cache them; paginate large grids, because even
  small images are expensive when there are many elements on screen at once.

> Origin: these rules were learned the hard way from an image-grid wallpaper
> picker (since removed) that froze the UI. The grid is gone, but the principle
> is general to any iced view. See also the brief note under "Performance
> Guidelines" in [`OVERVIEW.md`](../architecture/OVERVIEW.md).

---

## Widget idioms

### `widget::dropdown` — closures must be `'static`

`dropdown` takes a message-builder closure that must be `'static`, so it cannot
borrow local data. Clone anything the closure needs and `move` it in.

```rust
// src/app/visuals.rs — clone the data the closure captures, then `move` it
let themes = themes.clone();
let base_dropdown = widget::dropdown(names, selected_index, move |idx| {
    let registry_idx = themes[idx - 1].0.index; // captured by value
    Message::Page(/* ... */)
});
```

Signature is `dropdown(options, selected, on_select)` where `selected` is an
`Option<usize>` (`None::<usize>` for nothing selected).

### `file_chooser` — open/save dialogs

Use the xdg-portal-backed dialogs from `cosmic::dialog::file_chooser`. Distinct
`open::Dialog` and `save::Dialog` builders; always handle `Error::Cancelled`
separately from real errors.

```rust
// src/app/visuals.rs
use cosmic::dialog::file_chooser;

let dialog = file_chooser::open::Dialog::new()
    .title(/* ... */)
    .filter(file_chooser::FileFilter::new(&fl!("filter-ron-theme")).glob("*.ron"));

match dialog.open_file().await {
    Ok(response) => { /* response.url() -> file path */ }
    Err(file_chooser::Error::Cancelled) => { /* user dismissed — not an error */ }
    Err(e) => { /* real failure */ }
}
```

### Async work off the UI thread

Return a `cosmic::task::future` from `update()` to run slow work (file dialogs,
I/O, network) without blocking rendering; it resolves to a `Message` fed back
into `update()`.

```rust
cosmic::task::future(async move {
    let result = run_async_operation().await;
    Message::OperationComplete(result)
})
```

Used throughout `src/app/visuals.rs`, `src/app/screensaver.rs`, and
`src/app/idle.rs`.

---

## Theming: use the typed `cosmic_theme` API

Read and write COSMIC themes through the typed cosmic-config API in
`cosmic::cosmic_theme` rather than hand-editing the on-disk RON files:

```rust
// src/theme_config.rs
use cosmic::cosmic_theme::{Theme as CosmicTheme, ThemeBuilder, ThemeMode};

let cfg = CosmicTheme::dark_config()?;   // or light_config()
// ... build with ThemeBuilder, then persist via the config handle
```

The typed API handles serialization and is resilient to layout changes. For
reference, themes still land on disk under
`~/.config/cosmic/com.system76.CosmicTheme.{Dark,Light}/v1/` as RON, but treat
that as an implementation detail — prefer the API.

---

## Config paths live in one place

Every config file path is resolved through `src/paths.rs` (via
`directories::BaseDirs`, falling back to `$HOME/.config`). Don't hardcode paths
elsewhere — add a helper there. Current locations:

| What | Path | Helper |
|------|------|--------|
| App config dir | `~/.config/cosmic-order/` | `cosmic_order_config_dir()` |
| Tool sync config | `~/.config/cosmic-order/tool-sync.toml` | `tool_sync_config()` |
| Hooks | `~/.config/cosmic-order/hooks.d/` | `hooks_dir()` |
| Screensaver config | `~/.config/cosmic-screensaver/config` | `screensaver_config()` |
| Ghostty / btop / nvim / zellij / lazygit | their standard config dirs | see `paths.rs` |
| Bundled screensaver assets | `/usr/share/cosmic-order/screensaver/` | `screensaver_data_dir()` |

---

## Gotcha: editing `.ftl` translations may not rebuild

Translations are embedded at **compile time** via `RustEmbed` in
`src/localize.rs` (`#[derive(RustEmbed)] #[folder = "i18n/"]`). Cargo does not
always detect changes to `i18n/**/*.ftl` and may serve stale strings from an
incremental build.

If a translation change doesn't show up, force a re-embed:

```bash
cargo clean && cargo build
```

(Touching a `.rs` file in the same crate also works in a pinch.)
