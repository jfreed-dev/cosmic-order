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

| Component | Source | License |
|-----------|--------|---------|
| ASCII logos (`cosmic-*`) | Original | GPL-3.0-only |
| TTE effects | terminaltexteffects | MIT |
| Clock display | Original | GPL-3.0-only |

### ASCII Logo Status

Trademarked logos (Framework Computer, Pop!_OS / System76) were removed from
`resources/screensaver/logos/` in v0.15.0+. Only the project's original
`cosmic-*` logos ship in the repository and the `.deb` package. Users can
drop their own logo files into `~/.local/share/cosmic-order/screensaver/`.

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

---

Last updated: 2026-05-10
