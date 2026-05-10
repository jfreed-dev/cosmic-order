# Licensing & Attribution

This document tracks licensing requirements for COSMIC ORDER.

## Project License

**COSMIC ORDER**: GPL-3.0-only (matching COSMIC ecosystem)

## Code & Configuration Licensing

### Safe to Use ✅

| Component | Source | License | Attribution |
|-----------|--------|---------|-------------|
| OMARCHY patterns | basecamp/omarchy | MIT | Required |
| Tokyo Night colors | folke/tokyonight.nvim | MIT | Required |
| Catppuccin colors | catppuccin/catppuccin | MIT | Required |
| Gruvbox colors | morhetz/gruvbox | MIT | Required |
| Nord colors | nordtheme/nord | MIT | Required |
| Rose Pine colors | rose-pine/rose-pine-theme | MIT | Required |
| libcosmic | pop-os/libcosmic | MPL-2.0 | Required |

### Attribution Template

```text
Color schemes inspired by:
- OMARCHY by Basecamp (MIT License)
- Tokyo Night by folke (MIT License)
- Catppuccin by Catppuccin Org (MIT License)
- Gruvbox by morhetz (MIT License)
- Nord by Arctic Ice Studio (MIT License)
- Rose Pine by Rose Pine (MIT License)

Built with libcosmic by System76 (MPL-2.0)
```

## Theme Color Licensing

All color schemes are derived from open-source theme projects with MIT or
similar permissive licenses. Color values themselves are not copyrightable,
but attribution to original projects is good practice.

## Screensaver Components

| Component | Source | License | Trademark notes |
|-----------|--------|---------|-----------------|
| ASCII logos (`cosmic-*`) | Original ASCII art | GPL-3.0-only (file content) | Renders System76's COSMIC brand for nominative use; see below |
| TTE effects | terminaltexteffects | MIT | n/a |
| Clock display | Original | GPL-3.0-only | n/a |

### ASCII Logo Status

Trademarked logos for **unrelated brands** (Framework Computer, Pop!_OS) were
removed from `resources/screensaver/logos/` in v0.15.0+ — see commit
`31c53f3`. Only the `cosmic-*` logos remain; users can drop their own logo
files into `~/.local/share/cosmic-order/screensaver/`.

The surviving `cosmic-*` files render System76's **COSMIC** brand mark, which
is what COSMIC ORDER extends. The ASCII art itself is original work
distributable under GPL-3.0-only, but that license cannot grant any rights in
the underlying trademark — they are kept for **nominative use** (identifying
the desktop environment this software targets), and shipped with an
explicit affiliation disclaimer in `README.md` § "Trademarks" and a per-file
notice in `resources/screensaver/logos/LICENSES.md`.

## Public-Release Status

| Item | Status |
|------|--------|
| LICENSE file (full GPL-3.0 text) | ✅ |
| SPDX headers on all source files | ✅ |
| `Cargo.toml` `license = "GPL-3.0-only"` | ✅ |
| `debian/copyright` formatted with full GPL-3.0 trailer | ✅ |
| `metainfo.xml` `project_license = GPL-3.0-only` | ✅ |
| README attribution for OMARCHY + System76 | ✅ |
| README attribution for MIT theme color sources | ✅ |
| Trademarked ASCII logos (Framework, Pop!_OS) | ✅ Removed |
| README "Trademarks" disclaimer for System76 / COSMIC | ✅ |
| Per-file logo attribution (`resources/screensaver/logos/LICENSES.md`) | ✅ |

---

Last updated: 2026-05-10
