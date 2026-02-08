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

## Phase 3: Wallpaper Management

**Goal**: Organize and manage wallpapers with theme association.

### Tasks

- [x] Read wallpapers from system directories
- [x] Display wallpaper grid with thumbnails
- [x] Implement wallpaper selection
- [x] Add wallpaper to theme association
- [x] Implement wallpaper rotation configuration
- [x] Add wallpaper import (file picker)
- [ ] Add wallpaper download (URL)
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

## Phase 6: Tool Theme Synchronization

**Goal**: Unified theming across OMARCHY-style tools.

### Tasks

- [ ] Create colors.toml format specification (24-color standard)
- [ ] Build COSMIC theme → colors.toml converter
- [ ] Implement Ghostty theme generator
- [ ] Implement Neovim/LazyVim colorscheme generator
- [ ] Implement btop theme generator
- [ ] Implement Zellij theme generator
- [ ] Add CLI tools color export (fzf, lazygit)
- [ ] Create hook system for extensibility

### Deliverables

- Unified color format (colors.toml)
- Theme generators for all supported tools
- User-configurable tool selection

### Documentation

- colors.toml specification
- Adding new tool generators

---

## Phase 6A: Real-time Theme Sync

**Goal**: Instant theme propagation without restarts.

### Tasks

- [ ] Neovim RPC client for live theme updates
- [ ] Ghostty OSC escape sequences for color reload
- [ ] D-Bus integration for supporting apps
- [ ] File watcher for tools without IPC

### Deliverables

- Real-time Neovim theme switching
- Instant Ghostty color updates
- Seamless theme experience

---

## Phase 6B: Advanced Features

**Goal**: Additional features based on user feedback.

### Potential Features

- [ ] Theme creation wizard
- [ ] Online theme repository integration
- [ ] Wallpaper slideshow scheduling
- [ ] Per-workspace wallpapers
- [ ] Custom screensaver effects
- [ ] Panel applet for quick theme switching
- [ ] CLI interface for scripting

---

## Phase 6C: CPU Performance Management

**Goal**: Provide quick access to CPU turbo/performance settings.

### Tasks

- [ ] Detect system76-power availability
- [ ] Read current CPU performance profile
- [ ] Add CPU Turbo toggle button
- [ ] Display current CPU frequency/governor
- [ ] Integrate with system76-power profiles (performance/balanced/battery)
- [ ] Add power profile quick-switcher
- [ ] Show thermal status indicator (optional)

### Deliverables

- CPU Turbo on/off toggle
- Power profile switcher
- Current performance status display

### Documentation

- system76-power D-Bus interface
- CPU governor integration

## Phase 7: Deep Compositor Integration (Future)

**Goal**: Native COSMIC compositor integration for advanced screensaver features.

### Potential Features

- [ ] Native effect rendering (eliminate terminal dependency)
- [ ] Session lock protocol (ext-session-lock-v1)
- [ ] Layer-shell overlay surfaces
- [ ] Compositor-level cursor control
- [ ] Direct idle notification subscription
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
| 0.8.0 | 6A | Real-time theme sync |
| 0.8.5 | 6C | CPU performance management |
| 1.0.0 | 5 | First stable release |

## Success Criteria

Each phase is complete when:

1. All tasks are checked off
2. Documentation is updated
3. Code passes linting (`just check`)
4. Application builds without warnings
5. Features work on Pop!_OS with COSMIC Desktop
