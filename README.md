# COSMIC ORDER

**Establishing order in the chaos.**

OMARCHY-inspired workflow and aesthetics for COSMIC Desktop. We took the
keyboard-first philosophy, the curated applications, and the polished look -
and left the opinions behind.

Built with [libcosmic](https://github.com/pop-os/libcosmic) - the official
Rust toolkit for COSMIC applications.

## What Is This?

[OMARCHY](https://omarchy.com) is a cleverly crafted Linux distribution with
excellent taste in applications and workflow design. The keyboard-first
approach, the terminal aesthetics, the curated tool selection (ghostty, btop,
lazyvim) - it's genuinely good stuff.

COSMIC ORDER brings that workflow to [COSMIC Desktop](https://system76.com/cosmic),
letting you enjoy the rice without subscribing to the newsletter.

**Think of it as:** *"I'll take the workflow, hold the manifesto."*

## Project Status

| Status | Branch | Visibility |
|--------|--------|------------|
| **Alpha** | `alpha` | Private |

This project is in **alpha development**. See [docs/ROADMAP.md](docs/ROADMAP.md)
for the development plan.

## Goals

1. **Theme Management** - OMARCHY-inspired dark themes and accent colors
2. **Wallpaper Management** - Curated wallpapers with rotation support
3. **Screensaver** - Terminal-based screensaver with ASCII art effects
4. **Application Config** - Preconfigured settings for ghostty, btop, lazyvim
5. **Keyboard Shortcuts** - Keyboard-first workflow bindings for COSMIC

## Architecture

```text
┌─────────────────────────────────────────────────────────────┐
│                      COSMIC ORDER                           │
│              "Establishing order in the chaos"              │
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

## Acknowledgments

COSMIC ORDER draws heavy inspiration from [OMARCHY](https://omarchy.com),
created by DHH. We admire the curated workflow, application choices, and
keyboard-first philosophy that went into that project.

This project exists for those who appreciate the aesthetic and workflow
but prefer COSMIC Desktop as their foundation - and prefer their desktop
environment without editorial commentary.

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
