# COSMIC ORDER

[![License: GPL v3](https://img.shields.io/badge/license-GPL--3.0-blue.svg)](LICENSE)
[![Latest release](https://img.shields.io/github/v/release/jfreed-dev/cosmic-order)](https://github.com/jfreed-dev/cosmic-order/releases)
[![CI](https://github.com/jfreed-dev/cosmic-order/actions/workflows/ci.yml/badge.svg)](https://github.com/jfreed-dev/cosmic-order/actions/workflows/ci.yml)

OMARCHY-inspired workflow and aesthetics for COSMIC Desktop.

Built with [libcosmic](https://github.com/pop-os/libcosmic).

![COSMIC ORDER screensaver demo](docs/screensaver.gif)

> The terminal screensaver — the COSMIC mark revealed by one of
> TerminalTextEffects' beam effects on a blue→purple gradient.

## Features

- **Theme Management** — Browse, preview, and create COSMIC themes with a
  4-step creation wizard. 36 bundled themes (dark and light).
- **Tool Theme Sync** — Propagate COSMIC colors to Ghostty, btop, Neovim,
  Zellij, fzf, lazygit, and custom hooks. Auto-sync on theme change.
- **Screensaver** — Terminal-based screensaver with ASCII art effects,
  session lock, idle/DPMS timeout integration, and power-aware profiles.
- **Panel Applet** — A COSMIC panel button with a popup for quick screensaver
  control: lock now, start the screensaver, an enable/disable toggle, and open
  settings.
- **CLI** — Scriptable commands for theme switching, color extraction,
  tool sync, hooks, and wallpaper management.
- **Power Management** — Battery-aware screensaver effect profiles via UPower
  D-Bus / system76-power (Full/Standard/Simple/Minimal/Skip tiers). Idle
  inhibition ("caffeine") isn't bundled — use `systemd-inhibit` or an applet.

## Dependencies

### Build

```bash
# Pop!_OS / Ubuntu
sudo apt install cargo cmake just libexpat1-dev libfontconfig-dev \
  libfreetype-dev libxkbcommon-dev pkg-config
```

Rust 1.90+ required.

### Runtime

The app itself only needs its shared libraries (pulled in automatically by the
`.deb`). The **terminal screensaver** additionally uses:

| Tool | Purpose | Install |
|---|---|---|
| `alacritty` | default screensaver terminal (renders fullscreen) | `sudo apt install alacritty` |
| `terminaltexteffects` (`tte`) | the text-effect animations | `pipx install terminaltexteffects` (not packaged) |
| `swayidle` | idle detection that starts the screensaver | `sudo apt install swayidle` |
| `figlet` | *optional* — clock between effects | `sudo apt install figlet` |
| `ydotool` | *optional* — hides the mouse pointer | `sudo apt install ydotool` |

The `.deb` lists `alacritty`/`swayidle` under **Recommends** and
`figlet`/`ydotool` under **Suggests**; `tte` must be installed manually since it
isn't in the Debian/Ubuntu archive.

### Tool-sync targets (optional)

COSMIC ORDER writes your COSMIC theme into these tools' configs **when they are
installed** — install only the ones you use:

| Tool | Where to get it |
|---|---|
| Ghostty | <https://ghostty.org> |
| btop | `sudo apt install btop` |
| Neovim | `sudo apt install neovim` |
| fzf | `sudo apt install fzf` |
| Zellij | <https://zellij.dev> (release binary) |
| lazygit | <https://github.com/jesseduffield/lazygit/releases> (release binary) |

## Build

```bash
just              # build release
just run           # run with info logging
just check         # clippy pedantic
just test          # run tests
just pre-commit    # fmt + clippy + tests
```

## CLI Usage

```bash
cosmic-order sync                    # Sync colors to all enabled tools
cosmic-order colors --json           # Extract palette as JSON
cosmic-order theme dark              # Switch to dark mode
cosmic-order theme set-accent '#FF5733'
cosmic-order theme export theme.ron  # Export current theme
cosmic-order hooks run               # Run custom hooks
cosmic-order wallpaper add <url>     # Download a wallpaper
```

![COSMIC Screenshot](docs/COSMIC-screenshot.png)

## Install (.deb)

Download the latest `.deb` from the
[Releases page](https://github.com/jfreed-dev/cosmic-order/releases) and install
it:

```bash
sudo dpkg -i cosmic-order_*_amd64.deb
```

Or build it from source:

```bash
# Build the .deb package
dpkg-buildpackage -us -uc -b

# Install the resulting package
sudo dpkg -i ../cosmic-order_*.deb
```

Building requires `debhelper`, `just (>= 1.13.0)`, and `rust-all`.

## Known Upstream Bugs

See [docs/UPSTREAM-BUGS.md](docs/UPSTREAM-BUGS.md) for known fullscreen-related
bugs in upstream dependencies and workarounds.

## Acknowledgments

COSMIC ORDER draws inspiration from [OMARCHY](https://omarchy.com) by DHH.
The curated workflow, application choices, and keyboard-first philosophy
informed this project's design.

Built on [COSMIC Desktop](https://system76.com/cosmic) by System76, using
[libcosmic](https://github.com/pop-os/libcosmic) (MPL-2.0).

Bundled theme color schemes are derived from open-source projects under MIT
or compatible permissive licenses:

- [Tokyo Night](https://github.com/folke/tokyonight.nvim) by folke
- [Catppuccin](https://github.com/catppuccin/catppuccin) by the Catppuccin organization
- [Gruvbox](https://github.com/morhetz/gruvbox) by morhetz
- [Nord](https://www.nordtheme.com/) by Arctic Ice Studio
- [Rose Pine](https://rosepinetheme.com/) by the Rose Pine team

## Trademarks

COSMIC ORDER is **independent third-party software** and is **not
affiliated with or endorsed by System76**. "COSMIC" is a trademark of
System76, Inc.; the project name and the bundled `cosmic-*` ASCII
logos in `resources/screensaver/logos/` reference that mark to
identify the desktop environment this software extends. Trademarked
ASCII logos for unrelated brands (Framework, Pop!_OS) were removed in
v0.15.0; see [docs/LICENSING.md](docs/LICENSING.md) for the audit
trail.

If you are System76 and would like the project name, bundled logos,
or any of the integration language adjusted, please open an issue.

## License

Copyright © 2025-2026 Jonathan Freed

This project is licensed under the **GNU General Public License v3.0 only**
(GPL-3.0-only). See [LICENSE](LICENSE) for the full license text.

By submitting a pull request, you agree to license your contribution under
GPL-3.0-only.
