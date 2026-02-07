# Development Roadmap

This document outlines the phased development plan for COSMIC Tweaks.

## Overview

The project is divided into focused phases, each building on the previous.
Documentation is maintained alongside code development.

---

## Phase 0: Foundation ✅

**Goal**: Establish project structure, documentation, and development environment.

### Tasks

- [x] Research libcosmic architecture and capabilities
- [x] Research cosmic-settings patterns and conventions
- [x] Create project structure
- [x] Document research findings
- [x] Set up Rust project with Cargo
- [x] Configure clippy/rustfmt linting
- [x] Create justfile for build commands
- [x] Set up i18n with Fluent

### Deliverables

- [x] Project repository with documentation
- [x] Cargo.toml with all dependencies
- [x] Development environment setup guide

---

## Phase 1: Application Shell 🚧

**Goal**: Create the basic application structure with navigation.

### Tasks

- [x] Implement `cosmic::Application` trait
- [x] Create navigation sidebar with icons
- [x] Implement page routing system
- [x] Add placeholder pages (Themes, Wallpapers, Screensaver)
- [x] Set up cosmic-config integration (stubbed)
- [x] Implement i18n foundation with `fl!` macro
- [ ] Verify build compiles on Pop!_OS
- [ ] Test navigation works correctly
- [ ] Add application icon and desktop file
- [ ] Implement cosmic-config persistence

### Deliverables

- Application launches with navigation
- Pages switch correctly
- Configuration persists between sessions

### Documentation

- [x] Architecture overview
- [ ] Architecture decision records (ADRs)

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

**Goal**: Full GUI for the terminal-based screensaver system from
`laptop-configs-popos/screensaver`. Integrates with swayidle, TTE effects,
and provides a Caffeine-style idle inhibitor.

### Configuration Reading/Writing

- [ ] Parse `~/.config/cosmic-screensaver/config` (shell format)
- [ ] Write config changes back to file
- [ ] Call `screensaver-ctl reload` after changes
- [ ] Display service status (running/stopped)

### Timeout Settings

- [ ] Idle timeout slider (seconds before screensaver)
- [ ] Lock timeout slider (seconds after screensaver to lock, 0=disable)
- [ ] DPMS timeout slider (seconds to screen off, 0=disable)
- [ ] Battery-specific timeout (when on battery power)
- [ ] "Disable on battery" toggle

### Logo Selection

- [ ] List available logos from `screensaver/logos/` directory
- [ ] Preview logo ASCII art in-app
- [ ] Set logo via `screensaver-ctl logo <name>`
- [ ] Support custom logo import

### Effects Configuration

- [ ] List all TTE effects with checkboxes
- [ ] Toggle between include/exclude mode
- [ ] Add/remove effects from active list
- [ ] Frame rate slider (15-60 FPS)

### Fade Transitions

- [ ] Fade-in effect dropdown (expand, slide, middleout, etc.)
- [ ] Fade-out effect dropdown (burn, crumble, scattered, etc.)
- [ ] Enable/disable fades

### Clock Display

- [ ] Enable/disable clock between effects
- [ ] Clock duration slider
- [ ] Clock format dropdown (%H:%M, %H:%M:%S, %I:%M %p)
- [ ] Clock font selection (figlet fonts)

### Terminal Selection

- [ ] Radio buttons: Ghostty vs cosmic-term
- [ ] Dependency status indicator

### Service Management

- [ ] Enable/disable screensaver service button
- [ ] "Test Screensaver" button (launches preview)
- [ ] Service status indicator (green/red)

### Caffeine (Idle Inhibitor) ☕

- [ ] "Caffeine Mode" toggle in screensaver page
- [ ] When enabled: call D-Bus `org.freedesktop.ScreenSaver.Inhibit()`
- [ ] Status indicator showing inhibit is active
- [ ] Auto-disable after configurable timeout (optional)
- [ ] (Future) System tray applet for quick toggle

### Deliverables

- Full screensaver configuration GUI
- Preview capability
- Caffeine mode for temporary idle disable
- Service management (enable/disable)

### Documentation

- Screensaver configuration format reference
- Integration with swayidle
- D-Bus idle inhibitor protocol

---

## Phase 5: Polish and Integration

**Goal**: Refine the application for release.

### Accessibility & UX

- [ ] Keyboard navigation for all controls
- [ ] Screen reader labels (a11y)
- [ ] Consistent spacing and theming
- [ ] Loading states for async operations
- [ ] Error toasts for failures

### Internationalization

- [ ] Complete English translations
- [ ] Add Spanish translations
- [ ] Add German translations
- [ ] Translation contribution guide

### Error Handling

- [ ] Graceful fallbacks when scripts missing
- [ ] User-friendly error messages
- [ ] Logging with tracing levels

### Packaging

- [ ] Create .desktop file with icon
- [ ] AppStream metadata (for GNOME Software/Pop Shop)
- [ ] Debian package (.deb)
- [ ] Flatpak manifest (optional)

### Documentation

- [ ] User manual / README
- [ ] Installation guide
- [ ] Screenshots for README

### Deliverables

- Release-ready application
- Debian package
- User documentation
- AppStream listing

---

## Phase 6: Power Management

**Goal**: Advanced power controls that COSMIC doesn't expose natively.
Integrates with `laptop-configs-popos/scripts/cpu-power-mode.sh`.

### Power Profiles Integration

- [ ] Read current profile via `powerprofilesctl get`
- [ ] Display profile selector (Power Saver / Balanced / Performance)
- [ ] Set profile via `powerprofilesctl set <profile>`

### CPU Turbo Control

- [ ] Read turbo state from `/sys/devices/system/cpu/intel_pstate/no_turbo`
- [ ] "Turbo Boost" toggle switch
- [ ] Write via polkit helper (requires root)
- [ ] Show current turbo state indicator

### Performance Limits

- [ ] Max performance percentage slider (1-100%)
- [ ] Read/write `/sys/devices/system/cpu/intel_pstate/max_perf_pct`
- [ ] Show current CPU frequencies

### System Status Display

- [ ] Current power source (AC/Battery)
- [ ] CPU temperature (via lm-sensors)
- [ ] Sample CPU frequencies

### Polkit Integration

- [ ] Create polkit policy file for CPU controls
- [ ] Use `pkexec` or D-Bus for privileged writes
- [ ] Handle authorization failures gracefully

### Deliverables

- Power management page in COSMIC Tweaks
- Turbo boost toggle (the missing feature!)
- Performance limit controls
- System status display

### Documentation

- Intel P-State sysfs reference
- Polkit policy setup
- Integration with power-profiles-daemon

---

## Phase 7: Advanced Features (Future)

**Goal**: Additional features based on user feedback.

### Theme Features

- [ ] Theme creation wizard
- [ ] Online theme repository integration
- [ ] Theme scheduling (day/night auto-switch)

### Wallpaper Features

- [ ] Wallpaper slideshow scheduling
- [ ] Per-workspace wallpapers
- [ ] Wallpaper source plugins (Unsplash, etc.)

### Screensaver Features

- [ ] Custom TTE effect parameters
- [ ] User-uploadable logos with converter
- [ ] Screensaver scheduling

### System Integration

- [ ] Panel applet for quick toggles (theme/caffeine/turbo)
- [ ] CLI interface for scripting (`cosmic-tweaks set theme dark`)
- [ ] COSMIC Settings integration (if upstream accepts)

---

## Version Milestones

| Version | Phase | Description |
|---------|-------|-------------|
| 0.1.0 | 1 | Application shell with navigation |
| 0.2.0 | 2 | Theme management |
| 0.3.0 | 3 | Wallpaper management |
| 0.4.0 | 4 | Screensaver configuration + Caffeine |
| 0.5.0 | 6 | Power management |
| 1.0.0 | 5 | First stable release (polished) |

---

## Success Criteria

Each phase is complete when:

1. All tasks are checked off
2. Documentation is updated
3. Code passes linting (`cargo clippy`)
4. Application builds without warnings
5. Features work on Pop!_OS with COSMIC Desktop

---

## Reference: Existing Scripts

These scripts from `laptop-configs-popos` will be wrapped by COSMIC Tweaks:

| Script | Purpose | Phase |
|--------|---------|-------|
| `screensaver/screensaver-ctl.sh` | Screensaver management CLI | 4 |
| `screensaver/cosmic-screensaver.sh` | Main screensaver runner | 4 |
| `screensaver/launch-fullscreen.sh` | Fullscreen launcher | 4 |
| `scripts/cpu-power-mode.sh` | CPU/turbo control | 6 |
| `system/usr-local-bin/cpu-*` | Quick power toggles | 6 |
