# CLAUDE.md

Instructions for Claude Code when working on COSMIC ORDER.

## Project Overview

COSMIC ORDER is a native COSMIC Desktop application built with libcosmic (Rust).
OMARCHY-inspired workflow and aesthetics for COSMIC Desktop - the keyboard-first
workflow you love, on the desktop you deserve.

## Key Commands

```bash
# Build
cargo build --release

# Run
cargo run --release

# Run with debug logging
RUST_LOG=cosmic_order=debug cargo run

# Check code quality
cargo clippy --all-features

# Format code
cargo fmt

# Run tests
cargo test
```

## Architecture

- **Elm architecture**: State → View → Message → Update → State
- **libcosmic**: COSMIC's GUI toolkit built on iced
- **cosmic-config**: Configuration persistence

## Directory Structure

```text
src/
├── main.rs          # Entry point
├── app.rs           # Application state and routing
├── config.rs        # Configuration management
├── localize.rs      # i18n support
└── pages/
    └── mod.rs       # Page definitions and messages
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
