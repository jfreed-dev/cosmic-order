# COSMIC Tweaks

A native COSMIC Desktop application for managing themes, wallpapers, and
screensaver configurations on Pop!_OS and other COSMIC-based distributions.

Built with [libcosmic](https://github.com/pop-os/libcosmic) - the official
Rust toolkit for COSMIC applications.

## Project Status

| Status | Branch | Visibility |
|--------|--------|------------|
| **Alpha** | `alpha` | Private |

This project is in **alpha development** with a single developer workflow.
See [docs/ROADMAP.md](docs/ROADMAP.md) for the development plan and
[docs/development/WORKFLOW.md](docs/development/WORKFLOW.md) for the
release strategy.

## Goals

1. **Theme Management** - Create, edit, import/export COSMIC themes
2. **Wallpaper Management** - Organize wallpapers by theme with rotation support
3. **Screensaver Configuration** - Configure the terminal-based screensaver
4. **Integration** - Native COSMIC look and feel, integrates with system settings

## Architecture

```text
┌─────────────────────────────────────────────────────────────┐
│                     COSMIC Tweaks App                       │
├─────────────────────────────────────────────────────────────┤
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────┐ │
│  │   Themes    │  │ Wallpapers  │  │    Screensaver      │ │
│  │    Page     │  │    Page     │  │       Page          │ │
│  └─────────────┘  └─────────────┘  └─────────────────────┘ │
├─────────────────────────────────────────────────────────────┤
│                    libcosmic (Iced GUI)                     │
├─────────────────────────────────────────────────────────────┤
│  cosmic-config  │  cosmic-theme  │  cosmic-bg-config       │
└─────────────────────────────────────────────────────────────┘
```

## Requirements

- Pop!_OS 24.04+ with COSMIC Desktop
- Rust 1.85+
- System dependencies (see [docs/development/SETUP.md](docs/development/SETUP.md))

## Building

```bash
# Install dependencies (Pop!_OS/Ubuntu)
sudo apt install cargo cmake just libexpat1-dev libfontconfig-dev \
  libfreetype-dev libxkbcommon-dev pkg-config

# Build
cargo build --release

# Run
cargo run --release
```

## Documentation

| Document | Description |
|----------|-------------|
| [docs/ROADMAP.md](docs/ROADMAP.md) | Development phases and milestones |
| [docs/architecture/](docs/architecture/) | System design and architecture |
| [docs/research/](docs/research/) | Research on libcosmic and COSMIC |
| [docs/development/](docs/development/) | Development guides and setup |

## License

GPL-3.0-only (matching COSMIC ecosystem licensing)

## Contributing

This project follows COSMIC development practices. See
[docs/development/CONTRIBUTING.md](docs/development/CONTRIBUTING.md).
