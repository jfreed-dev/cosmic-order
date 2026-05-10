# Changelog

All notable changes to COSMIC ORDER are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.14.0] — 2026-05-10

First public release. Repository moved to
[`jfreed-dev/cosmic-order`](https://github.com/jfreed-dev/cosmic-order).

### Added

- `SECURITY.md`, `CODE_OF_CONDUCT.md`, root `CONTRIBUTING.md` for community
  health.
- Root `CHANGELOG.md` (this file).
- `authors` and `readme` fields in `Cargo.toml`.
- Theme color attribution and explicit copyright notice in `README.md`.
- `.deb` packaging now bundles screensaver scripts and ASCII logos.

### Changed

- Repository URL updated to `https://github.com/jfreed-dev/cosmic-order` in
  `Cargo.toml`, `debian/copyright`, and `metainfo.xml`.
- `docs/development/WORKFLOW.md` rewritten for public repository.
- `docs/LICENSING.md` checklist replaced with a status summary.
- `src/app.rs` split into focused modules; centralized config paths; DRY'd the
  theme generators and tool-sync pipeline.

### Removed

- Internal review notes (`prs/`).
- Pre-decision naming brainstorm (`docs/PROJECT-NAMING.md`).
- Personal session log (`docs/development/LEARNINGS.md`).
- Dead code and unused dependencies.

## [0.14.0-beta] — 2026-03-17

Beta stabilization release.

### Added

- CLI wallpaper download and management.
- Cosmictron theme set and expanded accent color presets.
- Theme creation wizard with live preview.
- Theme preview panel on Visuals page.
- Wallpaper preview panel with dual-tier thumbnail cache.
- Issue templates and PR template.

### Changed

- Visuals page reorganized: selectors on the left, preview on the right.
- Theme dropdowns replace card grids.
- Screensaver page split into Preview and Settings sections.

### Fixed

- Responsive Visuals layout.
- Theme dropdown source separation.

## [0.13.0] — 2026-03

Tool sync, CLI, and native session integration.

### Added

- CLI interface for scripted theme sync and color extraction (Phase 8).
- Native session lock via `ext-session-lock-v1` and logind D-Bus (Phase 7B).
- Native idle detection via Wayland (Phase 7A).
- Real-time theme propagation with live reload (Phase 6D).
- Tool theme sync for fzf, lazygit, with hook system (Phase 6C).
- Tool theme sync for btop, Neovim, Zellij (Phase 6B).
- Tool theme sync for Ghostty (Phase 6A).

### Fixed

- Logind-based screen lock replaces in-process session lock for reliability.
- Close button now works while native idle is active.

## [0.7.0] — 2026

Phase 5: packaging, polish, and async UI.

### Added

- Debian packaging infrastructure (Phase 5F).
- AppStream metadata and `install` target (Phase 5E).
- Tooltips on action buttons (Phase 5D).
- Async thumbnail generation for wallpaper grid (Phase 5C).
- Localization of all hardcoded strings (Phase 5A).

## Earlier

For changes prior to 0.7.0, see the git log.
