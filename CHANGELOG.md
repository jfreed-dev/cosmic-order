# Changelog

All notable changes to COSMIC ORDER are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.17.0] — 2026-05-22

### Added

- COSMIC panel applet (`cosmic-order-applet`): a panel button whose popup
  offers quick screensaver control — lock now, start the screensaver, an
  enable/disable toggle that reflects the idle-service state, and open
  settings. Actions reuse the existing surface (`loginctl`, the bundled
  screensaver scripts, `cosmic-order --page`). Ships an `X-CosmicApplet`
  desktop entry; add it from the panel's applet settings.

## [0.16.0] — 2026-05-22

### Added

- `cosmic-order --page <visuals|tool-sync|screensaver>` opens the GUI
  directly to a page (overriding the saved page) — used for deterministic
  multi-page screenshots.
- Alacritty support in the screensaver launcher, with a generated minimal
  `alacritty-screensaver.toml` (black background, no padding, hidden
  cursor, `startup_mode = "Fullscreen"`).
- `scripts/vm-test.sh` and the `just vm-test` recipe: build the amd64
  `.deb`, deploy it to a COSMIC VM, launch the GUI, and screenshot each
  page.
- Runtime-dependency documentation (README) and `debian/control`
  Recommends (alacritty, swayidle) / Suggests (figlet, ydotool).
- `.github/workflows/release.yml` — on a `v*` tag, builds the amd64
  `.deb` and publishes a GitHub Release with the package attached and
  notes drawn from this changelog.
- `.github/dependabot.yml` — weekly tracking of GitHub Actions versions
  in `.github/workflows/`. Cargo dependencies are intentionally left
  out (`just outdated` + the cargo-audit recipe already cover that
  surface).
- `.github/workflows/ci.yml` — GitHub Actions workflow that installs
  system deps, provisions Rust stable + `just`, caches cargo/target, and
  runs `just pre-commit` (fmt-check + clippy --pedantic + tests) on
  push/PR to `main`. Mirrors the gating pattern used by
  `jfreed-dev/niri-screensaver`.
- README banner badges (License GPL-3.0, Latest release, CI) directly
  under the H1, mirroring the niri-screensaver layout.
- README `Trademarks` section explicitly disclaiming affiliation with
  System76. "COSMIC" is a trademark of System76, Inc.; the project name
  and the surviving `cosmic-*` ASCII logos reference that mark for
  nominative use only.
- `resources/screensaver/logos/LICENSES.md` documents per-file
  attribution and trademark status for every remaining bundled logo,
  matching the pattern used in `jfreed-dev/niri-screensaver`.

### Changed

- Default screensaver terminal is now **Alacritty** (was Ghostty). It
  self-fullscreens via `startup_mode = "Fullscreen"`, avoiding the
  cosmic-term fullscreen freeze and Ghostty's ignored-at-startup
  fullscreen.
- Power-aware screensaver effect profiles are now **applied at runtime**:
  the driver reads `EFFECT_PROFILE` and feeds the matching effect list to
  `tte`. The slots were renamed to match the power tiers —
  **Full / Standard / Simple / Minimal**.
- `docs/LICENSING.md` § "ASCII Logo Status" now acknowledges that the
  surviving `cosmic-*` files render System76's COSMIC brand mark (the
  ASCII art content is GPL-3.0-only, but that license cannot grant
  rights in the underlying trademark — they're kept for nominative
  identification of the desktop COSMIC ORDER extends). The Public
  Release Status table gains a Trademarks-disclaimer row and a
  per-file-attribution row.

### Fixed

- Window title now reads "COSMIC ORDER" instead of "COSMIC".
- Screensaver no longer hangs on a `Password:` prompt during mouse
  parking (the `sg input` call blocked when the user wasn't in the
  `input` group).

### Removed

- Caffeine / idle-inhibitor feature. Use an external tool
  (`systemd-inhibit` or a COSMIC applet) for keep-awake.

## [0.15.0] — 2026-05-10

### Added

- COSMIC built-in themes selector (Dark, Light, High Contrast Dark, High
  Contrast Light) on the Visuals page (#9).
- Power Settings section on the Screensaver page: live UPower state,
  active power profile, system76-power availability, plus widgets for
  `disable_on_battery`, `battery_idle_timeout`, and the four per-profile
  effect overrides (#4).
- Clock font text input on the Screensaver page (#5).
- Warning banner on the Screensaver page when the bundled shell scripts
  (`launch-fullscreen.sh`, `screensaver-ctl.sh`, `cosmic-screensaver.sh`)
  are missing (#10).
- `--set` flag on `cosmic-order wallpaper add` to apply the downloaded
  image as the active wallpaper via `cosmic-bg`'s cosmic-config schema
  (#8).
- `nvim_colorscheme` field in `tool-sync.toml` (default `tokyonight`)
  used by both the Neovim generator and the live-reload remote-send
  command, so non-tokyonight setups stop hitting silent failures (#7).
- Sync results now flag tools with no live-reload mechanism (Zellij,
  fzf, lazygit) and the manual step the user must take. Surfaced both
  in the GUI status banner and as `apps_manual` in CLI JSON output
  (#6).
- Window size now persists across sessions and is restored on launch
  (#3).
- `docs/development/WORKFLOW.md` documents distribution packaging
  status: `.deb` shipped, Flatpak deferred with rationale (#12).

### Changed

- High-contrast theme presets are now applied via
  `CosmicTheme::write_entry` instead of only toggling dark mode, so
  selecting a high-contrast variant actually rewrites the active
  theme (#2).
- `tool_sync::signal_running_apps` returns `SignalResult { reloaded,
  skipped }` instead of `Vec<String>`. The CLI `sync --json` payload
  gains an `apps_manual` array.

### Removed

- Unused `Message::ConfigChanged` variant and its no-op handler (no
  cosmic-config subscription was ever wired up to construct it) (#11).

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
