# COSMIC ORDER

OMARCHY-inspired workflow and aesthetics for COSMIC Desktop.

Built with [libcosmic](https://github.com/pop-os/libcosmic).

## Features

- **Theme Management** — Browse, preview, and create COSMIC themes with a
  4-step creation wizard. 36 bundled themes (dark and light).
- **Tool Theme Sync** — Propagate COSMIC colors to Ghostty, btop, Neovim,
  Zellij, fzf, lazygit, and custom hooks. Auto-sync on theme change.
- **Screensaver** — Terminal-based screensaver with ASCII art effects,
  session lock, idle/DPMS timeout integration, and power-aware profiles.
- **CLI** — Scriptable commands for theme switching, color extraction,
  tool sync, hooks, and wallpaper management.
- **Power Management** — Battery-aware effect profiles via UPower D-Bus,
  caffeine mode (idle inhibitor) via logind.

## Dependencies

```bash
# Pop!_OS / Ubuntu
sudo apt install cargo cmake just libexpat1-dev libfontconfig-dev \
  libfreetype-dev libxkbcommon-dev pkg-config
```

Rust 1.90+ required.

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

## Install (.deb)

```bash
# Build the .deb package
dpkg-buildpackage -us -uc -b

# Install the resulting package
sudo dpkg -i ../cosmic-order_*.deb
```

Requires `debhelper`, `just (>= 1.13.0)`, and `rust-all` to build.

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

## License

Copyright © 2025-2026 Jonathan Freed

This project is licensed under the **GNU General Public License v3.0 only**
(GPL-3.0-only). See [LICENSE](LICENSE) for the full license text.

By submitting a pull request, you agree to license your contribution under
GPL-3.0-only.
