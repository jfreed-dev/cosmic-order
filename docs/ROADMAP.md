# Development Roadmap

This document outlines the phased development plan for COSMIC ORDER.

## Overview

The project is divided into focused phases, each building on the previous.
Documentation is maintained alongside code development.

## Phase 0: Foundation ✓

**Goal**: Establish project structure, documentation, and development environment.

### Tasks

- [x] Research libcosmic architecture and capabilities
- [x] Research cosmic-settings patterns and conventions
- [x] Create project structure
- [x] Document research findings
- [x] Set up Rust project with Cargo
- [x] Configure development environment
- [x] Create minimal "Hello COSMIC" application
- [x] Verify build on Pop!_OS with COSMIC

### Deliverables

- [x] Project repository with documentation
- [x] Working minimal libcosmic application
- [x] Development environment setup guide

---

## Phase 1: Application Shell ✓

**Goal**: Create the basic application structure with navigation.

### Tasks

- [x] Implement `cosmic::Application` trait
- [x] Create navigation sidebar
- [x] Implement page routing system
- [x] Add placeholder pages (Themes, Wallpapers, Screensaver)
- [x] Set up cosmic-config integration
- [x] Add application icon and desktop file
- [x] Implement i18n foundation

### Deliverables

- Application launches with navigation
- Pages switch correctly
- Configuration persists between sessions

### Documentation

- Architecture decision records (ADRs)
- Page system documentation

---

## Phase 2: Theme Management ✓

**Goal**: Implement theme viewing and basic customization.

### Tasks

- [x] Read system themes via cosmic-theme
- [x] Display theme list with previews
- [x] Implement theme switching (dark/light mode toggle)
- [x] Create color picker for accent colors
- [x] Add theme export functionality
- [x] Add theme import functionality
- [x] Implement theme preview (live preview before applying)

### Deliverables

- Users can view installed themes
- Users can switch between themes
- Users can customize accent colors
- Theme import/export works

### Documentation

- Theme file format documentation
- Integration with cosmic-theme

---

## Phase 3: Wallpaper Management ✓

**Goal**: Organize and manage wallpapers with theme association.

### Tasks

- [x] Read wallpapers from system directories
- [x] Display wallpaper grid with thumbnails
- [x] Implement wallpaper selection
- [x] Add wallpaper to theme association
- [x] Implement wallpaper rotation configuration
- [x] Add wallpaper import (file picker)
- [x] Add wallpaper download (URL)
- [x] Organize wallpapers by collection/theme

### Deliverables

- Users can browse wallpapers
- Users can set wallpaper
- Users can configure rotation
- Wallpapers organized by theme/collection

### Documentation

- Wallpaper storage locations
- Integration with cosmic-bg-config

---

## Phase 4: Screensaver Configuration ✓

**Goal**: Configure the terminal-based screensaver from the existing
`laptop-configs-popos/screensaver` implementation.

### Tasks

- [x] Read screensaver configuration
- [x] Display current screensaver settings
- [x] Configure idle timeout
- [x] Configure lock timeout
- [x] Configure DPMS timeout
- [x] Select screensaver logo
- [x] Configure effects (include/exclude)
- [x] Configure fade transitions
- [x] Configure clock display
- [x] Test screensaver preview
- [x] Enable/disable screensaver service

### Native Migration (NM-01 through NM-03) ✓

- [x] NM-01: Compositor settings via cosmic-config API
- [x] NM-02: DPMS timeout sync with cosmic-idle system config
- [x] NM-03: Native swayidle config generation + systemd D-Bus restart

See [NATIVE-MIGRATION.md](development/NATIVE-MIGRATION.md) for details.

### Deliverables

- Full screensaver configuration GUI
- Preview capability
- Service management (enable/disable)
- Native swayidle config generation (no shell subprocess for save/reload)

### Documentation

- Screensaver configuration format
- Integration with swayidle
- Native migration plan and status

---

## Phase 4A: Power-Aware Screensaver ✓

**Goal**: Integrate with system power management for intelligent behavior.

### Tasks

- [x] Add zbus and upower_dbus dependencies
- [x] Create power monitoring service
- [x] Detect AC power vs battery state
- [x] Read system76-power profile (performance/balanced/battery)
- [x] Create effect profiles for each power state
- [x] Adjust effect complexity based on power profile
- [x] Skip screensaver on critical battery (<10%)
- [x] Display power state in screensaver settings UI

### Deliverables

- Power-aware effect selection
- Battery threshold configuration
- Profile-specific effect lists

### Documentation

- Power integration architecture
- D-Bus interface usage

---

## Phase 4B: Enhanced Cursor & Input Handling ✓

**Goal**: Configurable cursor hiding and input dismiss behavior.

### Tasks

- [x] Add cursor_hide, hide_mouse, dismiss_on_key config fields
- [x] Add UI togglers in Cursor & Dismiss settings section
- [x] Make cursor/echo hiding conditional in cosmic-screensaver.sh
- [x] Make keyboard dismiss conditional in animation loop
- [x] Make mouse hiding conditional in Ghostty config generation
- [x] Add config defaults to screensaver-ctl.sh
- [x] Update tests for new fields (parse, roundtrip, defaults)
- [x] Reliable fullscreen via ydotool (kernel-level input injection)
- [x] Compositor interference management (autotile, focus_follows_cursor)
- [x] Mouse pointer hiding via ydotool mousemove
- [x] Mouse tracking dismiss (ESC sequence detection)
- [x] GUI Save & Test end-to-end verified

### Deliverables

- Configurable cursor hiding during screensaver
- Configurable keyboard dismiss behavior
- Configurable mouse pointer hiding
- Reliable fullscreen on COSMIC/Wayland via ydotool
- All settings persist through config file

---

## Phase 4C: Caffeine Mode (Idle Inhibitor) ✓

**Goal**: Allow users to temporarily prevent screen blanking and screensaver activation.

### Tasks

- [x] Create idle inhibitor module using logind D-Bus Inhibit API
- [x] Add Caffeine toggle button to main UI header
- [x] Visual indicator when active (selected state on header button)
- [x] Auto-disable Caffeine on low battery (<20%)

### Deliverables

- One-click idle inhibitor toggle in header
- Visual indicator when active (button selected state + tooltip)
- Battery-aware auto-disable

## Phase 5: Polish and Integration ✓

**Goal**: Refine the application for release.

### Tasks

- [x] i18n — Localize remaining hardcoded strings (file dialog filters, timeout display)
- [x] Error handling review — Add debug logging to silent D-Bus downcasts
- [x] Performance optimization — Async thumbnail generation (background tasks)
- [x] Accessibility — Add tooltips to all action buttons
- [x] Create AppStream metadata (metainfo.xml)
- [x] Create installation package (Debian packaging)
- [x] Version bump to 0.7.0

### Deliverables

- All user-visible strings localized via fl!()
- Async thumbnail generation (no I/O in view())
- Button tooltips for keyboard/accessibility
- AppStream metainfo.xml + install target
- Debian packaging infrastructure (debian/)
- Version 0.7.0

---

## Phase 6A: Tool Theme Synchronization — colors.toml + Ghostty ✓

**Goal**: Extract COSMIC theme colors and sync to Ghostty terminal.

### Tasks

- [x] Create ColorPalette struct with COSMIC theme extraction
- [x] Implement colors.toml serialization (OMARCHY format)
- [x] Implement Ghostty theme generator + activation
- [x] Create tool sync orchestration with per-tool enable/disable
- [x] Add Tool Sync UI section to Themes page
- [x] Localize all tool sync strings
- [x] Version bump to 0.8.0

### Deliverables

- `~/.config/cosmic-order/colors.toml` — 22-color palette from COSMIC theme
- `~/.config/ghostty/themes/cosmic-synced` — Ghostty theme file
- `~/.config/ghostty/config` — auto-activates cosmic-synced theme
- Tool Sync settings section with per-tool toggles and Sync Now button

---

## Phase 6B: Additional Tool Generators ✓

**Goal**: Extend theme sync to Neovim, btop, and Zellij.

### Tasks

- [x] Implement btop theme generator
- [x] Implement Neovim/LazyVim colorscheme generator
- [x] Implement Zellij theme generator
- [x] Extend tool sync orchestration with per-tool enable/disable
- [x] Add per-tool togglers in Tool Sync UI
- [x] Localize all new tool sync strings
- [x] Version bump to 0.9.0

### Deliverables

- `~/.config/btop/themes/cosmic-synced.theme` — btop theme with gradient colors
- `~/.config/nvim/lua/plugins/colorscheme.lua` — tokyonight with COSMIC colors
- `~/.config/zellij/config.kdl` — Zellij theme block with decimal RGB colors
- Per-tool togglers and sync for all 4 tools (Ghostty, btop, Neovim, Zellij)

---

## Phase 6C: CLI Tools + Hook System ✅

**Goal**: CLI tool color export and extensibility.

### Tasks

- [x] Add fzf theme generator with shell integration toggle
- [x] Add lazygit theme generator with comment-marker config update
- [x] Create hook system (`~/.config/cosmic-order/hooks.d/`)

### Deliverables

- `~/.config/cosmic-order/fzf-theme.sh` — fzf color theme (shell source)
- `~/.config/lazygit/config.yml` — lazygit gui.theme block
- `~/.config/cosmic-order/hooks.d/` — user-defined hook scripts
- Per-tool togglers for fzf, lazygit, and custom hooks

---

## Phase 6D: Real-time Propagation ✅ (v0.11.0)

**Goal**: Real-time theme propagation when COSMIC desktop theme changes.

### Completed

- [x] Auto-sync on COSMIC theme change via `system_theme_update()` / `system_theme_mode_update()`
- [x] Live reload of Ghostty via SIGUSR2
- [x] Live reload of btop via SIGUSR2
- [x] Live reload of Neovim via `--remote-send` to unix sockets
- [x] "Auto-sync on theme change" toggler (opt-in, default off)

### Future Ideas

- [ ] Theme creation wizard
- [ ] Online theme repository integration
- [ ] Wallpaper slideshow scheduling
- [ ] Per-workspace wallpapers
- [ ] Custom screensaver effects
- [ ] Panel applet for quick theme switching
- [x] CLI interface for scripting (Phase 8)
- [ ] CPU performance management (system76-power integration)

## Phase 7A: Native Idle Detection ✅ (v0.12.0)

**Goal**: Direct compositor idle detection via ext-idle-notify-v1 Wayland protocol.

### Completed

- [x] Wayland idle subscription via ext-idle-notify-v1 (screensaver + lock timeouts)
- [x] Lock-before-suspend via logind PrepareForSleep D-Bus signal
- [x] Automatic swayidle fallback (stop on connect, restart on exit/error)
- [x] Config-driven subscription restart (timeout changes take effect immediately)
- [x] Caffeine mode respected for native idle events
- [x] `on_app_exit()` + `Drop` safety net for swayidle restart

## Phase 7B: Session Lock ✅ (v0.12.0)

**Goal**: Lock the screen after idle timeout using COSMIC's greeter.

### Completed

- [x] Timer-based lock scheduling (cancellable `Task::abortable` on screensaver start)
- [x] Screen lock via logind D-Bus (`loginctl lock-session` → COSMIC greeter)
- [x] `screensaver_config.session_lock` field with UI toggle in Screensaver page
- [x] Lock timer cancelled on user resume

### Findings

- In-process ext-session-lock-v1 is **not viable** — acquiring the lock disrupts
  the main app's Wayland connection (broken pipe), crashing while locked
- Fullscreen screensaver window resets compositor idle timer — Wayland idle lock
  notification unreliable after screensaver starts
- `src/session_lock.rs` deleted (non-viable approach, no longer needed)

## Phase 8: CLI Interface ✅ (v0.13.0)

**Goal**: Scriptable CLI for theme sync, color extraction, and theme switching.

### Completed

- [x] Single binary: `cosmic-order` with no args launches GUI, subcommands run CLI mode
- [x] `sync [--reload] [--json]` — full theme sync pipeline
- [x] `colors [--json]` — extract palette to stdout (TOML or JSON)
- [x] `colors save [path]` — save colors.toml to disk
- [x] `theme info [--json]` — show current theme name, mode, accent
- [x] `theme dark` / `theme light` — switch mode
- [x] `theme set-accent <hex>` — set accent color
- [x] `theme export <path>` / `theme import <path>` — .ron theme export/import
- [x] `hooks run [--json]` — run all hooks with current palette
- [x] Human-readable output by default, `--json` for scripting
- [x] All strings localized via `fl!()`
- [x] Added `clap` (derive) and `serde_json` dependencies

---

## Phase 7: Deep Compositor Integration (Future)

**Goal**: Native COSMIC compositor integration for advanced screensaver features.

### Potential Features

- [ ] Native effect rendering (eliminate terminal dependency)
- [ ] Standalone session lock binary (ext-session-lock-v1 with PAM authentication)
- [ ] Layer-shell overlay surfaces
- [ ] Compositor-level cursor control
- [x] Direct idle notification subscription (Phase 7A)
- [x] Session lock via logind D-Bus (Phase 7B)
- [ ] cosmic-term contributions (--fullscreen flag)

### Research Required

- Smithay framework for Wayland protocols
- cosmic-comp integration patterns
- Layer-shell protocol usage

See [SCREENSAVER-INTEGRATION.md](SCREENSAVER-INTEGRATION.md) for detailed research.

---

## Version Milestones

| Version | Phase | Description |
|---------|-------|-------------|
| 0.1.0 | 1 | Application shell with navigation |
| 0.2.0 | 2 | Theme management |
| 0.3.0 | 3 | Wallpaper management |
| 0.4.0 | 4 | Screensaver configuration |
| 0.5.0 | 4A | Power-aware screensaver |
| 0.6.0 | 4B | Enhanced cursor/input handling |
| 0.6.5 | 4C | Caffeine mode (idle inhibitor) |
| 0.7.0 | 5 | Polish and integration |
| 0.8.0 | 6A | Tool theme sync (colors.toml + Ghostty) |
| 0.9.0 | 6B | Additional tool generators (btop, Neovim, Zellij) |
| 0.10.0 | 6C | CLI tools + hook system (fzf, lazygit) |
| 0.11.0 | 6D | Real-time theme propagation |
| 0.12.0 | 7A | Native idle detection |
| 0.12.1 | 7B | Session lock via logind D-Bus |
| 0.13.0 | 8 | CLI interface for scripting |
| 0.14.0-beta | Beta | Stabilization, cleanup, wallpaper URL download |
| 1.0.0 | 7 | First stable release |

## Success Criteria

Each phase is complete when:

1. All tasks are checked off
2. Documentation is updated
3. Code passes linting (`just check`)
4. Application builds without warnings
5. Features work on Pop!_OS with COSMIC Desktop
