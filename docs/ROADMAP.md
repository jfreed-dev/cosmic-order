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

## Phase 1: Application Shell (Current)

**Goal**: Create the basic application structure with navigation.

### Tasks

- [x] Implement `cosmic::Application` trait
- [x] Create navigation sidebar
- [x] Implement page routing system
- [x] Add placeholder pages (Themes, Wallpapers, Screensaver)
- [ ] Set up cosmic-config integration
- [ ] Add application icon and desktop file
- [x] Implement i18n foundation

### Deliverables

- Application launches with navigation
- Pages switch correctly
- Configuration persists between sessions

### Documentation

- Architecture decision records (ADRs)
- Page system documentation

---

## Phase 2: Theme Management

**Goal**: Implement theme viewing and basic customization.

### Tasks

- [ ] Read system themes via cosmic-theme
- [ ] Display theme list with previews
- [ ] Implement theme switching
- [ ] Create color picker for accent colors
- [ ] Add theme export functionality
- [ ] Add theme import functionality
- [ ] Implement theme preview (live preview before applying)

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

- [ ] Read wallpapers from system directories
- [ ] Display wallpaper grid with thumbnails
- [ ] Implement wallpaper selection
- [ ] Add wallpaper to theme association
- [ ] Implement wallpaper rotation configuration
- [ ] Add wallpaper import (file picker)
- [ ] Add wallpaper download (URL)
- [ ] Organize wallpapers by collection/theme

### Deliverables

- Users can browse wallpapers
- Users can set wallpaper
- Users can configure rotation
- Wallpapers organized by theme/collection

### Documentation

- Wallpaper storage locations
- Integration with cosmic-bg-config

---

## Phase 4: Screensaver Configuration

**Goal**: Configure the terminal-based screensaver from the existing
`laptop-configs-popos/screensaver` implementation.

### Tasks

- [ ] Read screensaver configuration
- [ ] Display current screensaver settings
- [ ] Configure idle timeout
- [ ] Configure lock timeout
- [ ] Configure DPMS timeout
- [ ] Select screensaver logo
- [ ] Configure effects (include/exclude)
- [ ] Configure fade transitions
- [ ] Configure clock display
- [ ] Test screensaver preview
- [ ] Enable/disable screensaver service

### Deliverables

- Full screensaver configuration GUI
- Preview capability
- Service management (enable/disable)

### Documentation

- Screensaver configuration format
- Integration with swayidle

---

## Phase 4A: Power-Aware Screensaver

**Goal**: Integrate with system power management for intelligent behavior.

### Tasks

- [ ] Add zbus and upower_dbus dependencies
- [ ] Create power monitoring service
- [ ] Detect AC power vs battery state
- [ ] Read system76-power profile (performance/balanced/battery)
- [ ] Create effect profiles for each power state
- [ ] Adjust effect complexity based on power profile
- [ ] Skip screensaver on critical battery (<10%)
- [ ] Display power state in screensaver settings UI

### Deliverables

- Power-aware effect selection
- Battery threshold configuration
- Profile-specific effect lists

### Documentation

- Power integration architecture
- D-Bus interface usage

---

## Phase 4B: Enhanced Cursor & Input Handling

**Goal**: Reliable cursor hiding and input wake detection.

### Tasks

- [ ] Verify fullscreen cursor behavior on COSMIC
- [ ] Document actual cursor hiding behavior
- [ ] Implement pointer confinement if needed
- [ ] Ensure clean cursor restore on wake
- [ ] Test wake on mouse movement
- [ ] Test wake on keyboard input
- [ ] Add configurable wake sensitivity

### Deliverables

- Cursor reliably hidden during screensaver
- Clean input wake handling
- Configurable behavior

---

## Phase 5: Polish and Integration

**Goal**: Refine the application for release.

### Tasks

- [ ] Accessibility review (a11y)
- [ ] Keyboard navigation
- [ ] Complete i18n translations
- [ ] Performance optimization
- [ ] Error handling review
- [ ] Create installation package (deb)
- [ ] Write user documentation
- [ ] Create AppStream metadata

### Deliverables

- Release-ready application
- Debian package
- User documentation

### Documentation

- Installation guide
- User manual

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
| 0.7.0 | 6 | Tool theme synchronization |
| 0.8.0 | 6A | Real-time theme sync |
| 1.0.0 | 5 | First stable release |

## Success Criteria

Each phase is complete when:

1. All tasks are checked off
2. Documentation is updated
3. Code passes linting (`cargo clippy`)
4. Application builds without warnings
5. Features work on Pop!_OS with COSMIC Desktop
