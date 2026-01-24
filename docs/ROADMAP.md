# Development Roadmap

This document outlines the phased development plan for COSMIC ORDER.

## Overview

The project is divided into focused phases, each building on the previous.
Documentation is maintained alongside code development.

## Phase 0: Foundation (Current)

**Goal**: Establish project structure, documentation, and development environment.

### Tasks

- [x] Research libcosmic architecture and capabilities
- [x] Research cosmic-settings patterns and conventions
- [x] Create project structure
- [x] Document research findings
- [ ] Set up Rust project with Cargo
- [ ] Configure development environment
- [ ] Create minimal "Hello COSMIC" application
- [ ] Verify build on Pop!_OS with COSMIC

### Deliverables

- Project repository with documentation
- Working minimal libcosmic application
- Development environment setup guide

---

## Phase 1: Application Shell

**Goal**: Create the basic application structure with navigation.

### Tasks

- [ ] Implement `cosmic::Application` trait
- [ ] Create navigation sidebar
- [ ] Implement page routing system
- [ ] Add placeholder pages (Themes, Wallpapers, Screensaver)
- [ ] Set up cosmic-config integration
- [ ] Add application icon and desktop file
- [ ] Implement i18n foundation

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

## Phase 6: Advanced Features (Future)

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

## Version Milestones

| Version | Phase | Description |
|---------|-------|-------------|
| 0.1.0 | 1 | Application shell with navigation |
| 0.2.0 | 2 | Theme management |
| 0.3.0 | 3 | Wallpaper management |
| 0.4.0 | 4 | Screensaver configuration |
| 1.0.0 | 5 | First stable release |

## Success Criteria

Each phase is complete when:

1. All tasks are checked off
2. Documentation is updated
3. Code passes linting (`cargo clippy`)
4. Application builds without warnings
5. Features work on Pop!_OS with COSMIC Desktop
