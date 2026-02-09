# CLAUDE.md

Instructions for Claude Code when working on COSMIC ORDER.

## Project Overview

COSMIC ORDER is a native COSMIC Desktop application built with libcosmic (Rust).
OMARCHY-inspired workflow and aesthetics for COSMIC Desktop - the keyboard-first
workflow you love, on the desktop you deserve.

## Key Commands

Uses `just` (follows cosmic-app-template conventions):

```bash
# Build
just                    # build-release (default)
just build-debug        # debug profile

# Run
just run                # release with info logging
just run-debug          # debug build with debug logging
just run-trace          # trace logging

# Quality
just check              # clippy (pedantic)
just fmt                # format code
just fmt-check          # check formatting
just lint               # clippy + fmt-check + doc warnings
just test               # run tests
just pre-commit         # fmt-check + clippy + tests

# Install / Package
just install            # install binary, desktop file, icon
just uninstall          # remove installed files
just vendor             # vendor dependencies for offline builds
just build-vendored     # build with vendored deps
```

## Architecture

- **Elm architecture**: State → View → Message → Update → State
- **libcosmic**: COSMIC's GUI toolkit built on iced
- **cosmic-config**: Configuration persistence

## Directory Structure

```text
src/
├── main.rs              # Entry point, APP_ID
├── app.rs               # Application state, routing, all page views
├── config.rs            # Configuration management (cosmic-config)
├── localize.rs          # i18n support (fl! macro)
├── pages/
│   └── mod.rs           # Page IDs, message enums
├── theme_config.rs      # Theme reading/writing, preview, export/import
├── wallpaper_config.rs  # Wallpaper config, thumbnails, RON structs
├── screensaver_config.rs # Screensaver config parsing/generation
├── colors.rs            # ColorPalette extraction, colors.toml generation
├── generators/
│   ├── mod.rs           # Generator module declarations
│   └── ghostty.rs       # Ghostty theme generator
├── tool_sync.rs         # Tool sync orchestration and config
├── compositor.rs        # COSMIC compositor settings
├── cosmic_idle.rs       # DPMS timeout sync
├── inhibit.rs           # Idle inhibitor (caffeine mode)
├── power.rs             # Power monitoring (UPower D-Bus)
└── systemd.rs           # Systemd D-Bus unit restart
```

## Code Standards

### Licensing

All source files must include:

```rust
// SPDX-License-Identifier: GPL-3.0-only
```

### Error Handling

Never use `.unwrap()` or `.expect()` in production code:

```rust
// Bad
let value = config.get("key").unwrap();

// Good
match config.get("key") {
    Ok(value) => { /* use value */ }
    Err(e) => tracing::error!("Failed: {e}"),
}
```

### Logging

Use tracing macros:

```rust
tracing::info!("Starting operation");
tracing::debug!("Debug info: {:?}", data);
tracing::warn!("Warning condition");
tracing::error!("Error occurred: {}", err);
```

### Theming

Always use theme values for spacing:

```rust
let spacing = cosmic::theme::spacing();
widget::column()
    .spacing(spacing.space_s)
    .padding(spacing.space_m)
```

### Localization

Use the `fl!` macro for all user-facing strings:

```rust
use crate::fl;

widget::text::title2(fl!("page-title"))
```

## Documentation

- [docs/ROADMAP.md](docs/ROADMAP.md) - Development phases
- [docs/architecture/](docs/architecture/) - System design
- [docs/research/](docs/research/) - libcosmic and COSMIC research
- [docs/development/](docs/development/) - Development guides
- [docs/development/WORKFLOW.md](docs/development/WORKFLOW.md) - Git workflow
- [docs/development/NATIVE-MIGRATION.md](docs/development/NATIVE-MIGRATION.md) - Native API
  migration plan

## Development Phase

**Current: Alpha** (private repo, single developer, direct commits to `alpha`)

```bash
# Standard workflow
git checkout alpha
# make changes
git add -A && git commit -m "feat: description"
git push
```

See WORKFLOW.md for beta/public release plans.

## Related Projects

- [libcosmic](https://github.com/pop-os/libcosmic) - GUI toolkit
- [cosmic-settings](https://github.com/pop-os/cosmic-settings) - Reference app
- [laptop-configs-popos](../laptop-configs-popos) - Screensaver scripts
- [OMARCHY](https://omarchy.com) - Workflow inspiration

## AI Attribution Policy

Never reference AI tools in commits, comments, or documentation.
All work must appear as natural developer output.
