# Testing Guide

Comprehensive testing strategy for COSMIC ORDER: unit tests, health checks,
and user acceptance testing.

## Quick Reference

```bash
# Unit tests
just test                    # Run all tests
just test-verbose            # Run with output
just pre-commit              # fmt + clippy + tests

# Health checks
./scripts/health-check.sh          # Full system check
./scripts/health-check.sh --quick  # Build checks only (no runtime)
```

## Unit Tests

### Running Tests

```bash
cargo test                          # All tests
cargo test cli::tests               # Single module
cargo test test_cli_sync            # Single test by name
cargo test -- --nocapture           # Show println output
```

### Test Coverage by Module

| Module | Tests | What's Tested |
|---|---|---|
| `colors` | 7 | TOML format, hex validation, hex-to-RGB, srgb/srgba hex, pack/unpack |
| `tool_sync` | 3 | Default config, TOML roundtrip, backwards compat |
| `screensaver_config` | 5 | Parse, defaults, roundtrip, swayidle generation |
| `bundled_themes` | 3 | Filename parsing, all themes load, dark/light split |
| `ghostty` | 2 | Theme format, no font settings leak |
| `btop` | 2 | Theme format, key count |
| `fzf` | 4 | Key count, color mapping, shell validity, source line |
| `nvim` | 4 | Lua validity, color assignment, darken/lighten |
| `lazygit` | 6 | YAML format, colors, key count, config updates |
| `zellij` | 6 | KDL format, structure, decimal RGB, config updates |
| `hooks` | 4 | Env var count/content, directory path, defaults |
| `power` | 4 | Effect profiles, env format, display, defaults |
| `paths` | 2 | Path resolution, prefix consistency |
| `cli` | 24 | Argument parsing, subcommands, flags, JSON output |
| `config` | 6 | Defaults, clone, equality, serialization, errors |
| `pages` | 7 | PageId enum, serialization, message construction |

**Total: 91 tests**

### Modules Without Unit Tests

These modules interact directly with system services and are tested through
the health check script and UAT procedures instead:

- `compositor.rs` - COSMIC compositor config (requires cosmic-config)
- `cosmic_idle.rs` - DPMS timeout sync (requires cosmic-config)
- `systemd.rs` - Systemd D-Bus calls (requires session bus)
- `sleep_lock.rs` - Session lock monitoring (requires logind D-Bus)
- `wayland_idle.rs` - Wayland idle protocol (requires Wayland session)
- `app/` - GUI application (requires COSMIC runtime)
- `theme_config.rs` - Theme config (requires cosmic-config runtime)
- `localize.rs` - i18n initialization (tested implicitly by all `fl!()` usage)

## Health Check Script

`./scripts/health-check.sh` validates build integrity, runtime dependencies,
and system integration.

### Quick Mode (CI/Build Verification)

```bash
./scripts/health-check.sh --quick
```

Checks: Rust version, formatting, clippy, compilation, tests, SPDX headers,
APP_ID consistency, i18n, bundled themes.

### Full Mode (System Integration)

```bash
./scripts/health-check.sh
```

Additional checks: D-Bus session/system bus, Wayland session, COSMIC Desktop
detection, cosmic-config directory, tool-sync config, hooks directory, installed
sync targets (ghostty, btop, nvim, etc.), CLI smoke tests (--version, --help,
colors output, JSON validity).

### Exit Codes

- `0` - All checks passed (warnings are informational)
- `1` - One or more checks failed

## User Acceptance Testing (UAT)

### Prerequisites

- COSMIC Desktop session (Wayland)
- At least one sync target installed (ghostty, btop, nvim, zellij, fzf, lazygit)
- Built release binary: `just` or `cargo build --release`

### UAT-01: Application Launch

| Step | Action | Expected |
|---|---|---|
| 1 | Run `just run` | App window opens within 15s |
| 2 | Observe window | Three nav pages visible: Visuals, Tool Sync, Screensaver |
| 3 | Click each page | Page content loads, no errors in terminal |
| 4 | Close window | App exits cleanly, no crash output |

### UAT-02: Theme Browsing and Preview

| Step | Action | Expected |
|---|---|---|
| 1 | Navigate to Visuals page | Theme grid shows bundled themes |
| 2 | Click a theme thumbnail | Theme preview applies immediately (desktop colors change) |
| 3 | Click "Cancel" / press Escape | Previous theme restores |
| 4 | Click a theme, then "Confirm" | Theme persists, preview dismissed |
| 5 | Switch between dark and light themes | Mode toggles correctly |

### UAT-03: Theme Creation Wizard

| Step | Action | Expected |
|---|---|---|
| 1 | Click "Create Theme" button | Wizard opens at Step 1 (base theme) |
| 2 | Select a base theme | Theme applies as preview |
| 3 | Click Next → accent color step | Accent presets and hex input visible |
| 4 | Click a color preset | Accent changes live |
| 5 | Type a hex value (e.g. `#FF5733`) | Accent updates on valid hex |
| 6 | Continue through remaining steps | Gap/hint/radius/frosted options work |
| 7 | Enter a theme name, click Apply | Theme saved, wizard closes |
| 8 | Click Close at any step | Wizard closes, original theme restores |

### UAT-04: Tool Sync

| Step | Action | Expected |
|---|---|---|
| 1 | Navigate to Tool Sync page | Toggle switches for each tool |
| 2 | Enable Ghostty sync (if installed) | Toggle turns on |
| 3 | Click "Sync Now" | Success message appears |
| 4 | Verify Ghostty config | `~/.config/ghostty/cosmic-synced` contains theme colors |
| 5 | Enable Auto Sync | Toggle turns on |
| 6 | Change theme on Visuals page | Tool configs update automatically |
| 7 | Open synced tool (e.g. ghostty) | Colors match COSMIC theme |

### UAT-05: CLI Interface

```bash
# Theme info
cosmic-order theme info
cosmic-order theme info --json

# Color extraction
cosmic-order colors
cosmic-order colors --json
cosmic-order colors save /tmp/test-colors.toml
cat /tmp/test-colors.toml  # Verify TOML format

# Theme switching
cosmic-order theme dark      # Switch to dark mode
cosmic-order theme light     # Switch to light mode
cosmic-order theme set-accent '#FF5733'

# Theme export/import
cosmic-order theme export /tmp/test-theme.ron
cosmic-order theme import /tmp/test-theme.ron

# Tool sync
cosmic-order sync
cosmic-order sync --json
cosmic-order sync --reload

# Hooks
cosmic-order hooks run
cosmic-order hooks run --json
```

| Step | Action | Expected |
|---|---|---|
| 1 | Run each command above | Exit code 0, correct output format |
| 2 | Verify `--json` output | Valid JSON (pipe to `jq .`) |
| 3 | Verify theme switching | Desktop theme changes |
| 4 | Verify export/import | .ron file created/applied |

### UAT-06: Screensaver Configuration

| Step | Action | Expected |
|---|---|---|
| 1 | Navigate to Screensaver page | All settings visible with current values |
| 2 | Toggle Enable on | Screensaver enabled |
| 3 | Set idle timeout to 60s | Value updates |
| 4 | Set lock timeout to 120s | Value updates |
| 5 | Click "Save & Test" | Screensaver launches fullscreen |
| 6 | Press any key | Screensaver dismisses (if dismiss-on-key enabled) |
| 7 | Verify swayidle config | `~/.config/cosmic-screensaver/swayidle.conf` matches settings |

### UAT-07: Power Management

| Step | Action | Expected |
|---|---|---|
| 1 | Run on battery | Power indicator shows battery status |
| 2 | Observe effect profile | Screensaver effects adjust for battery level |
| 3 | Plug in AC | Status updates to AC power |
| 4 | Verify no excessive CPU | `htop` shows low idle CPU usage |

### UAT-08: Custom Hooks

```bash
# Create a test hook
mkdir -p ~/.config/cosmic-order/hooks.d
cat > ~/.config/cosmic-order/hooks.d/test-hook.sh << 'HOOK'
#!/bin/bash
echo "Hook fired! Accent: $COSMIC_ACCENT"
echo "Colors file: $1"
HOOK
chmod +x ~/.config/cosmic-order/hooks.d/test-hook.sh
```

| Step | Action | Expected |
|---|---|---|
| 1 | Create test hook (above) | File created and executable |
| 2 | Run `cosmic-order hooks run` | Output shows hook succeeded |
| 3 | Run `cosmic-order hooks run --json` | JSON shows `hooks_run: 1, hooks_succeeded: 1` |
| 4 | Enable hooks in Tool Sync page | Toggle turns on |
| 5 | Click Sync Now | Hook runs as part of sync |
| 6 | Remove test hook | `rm ~/.config/cosmic-order/hooks.d/test-hook.sh` |

### UAT-09: Wallpaper Download

| Step | Action | Expected |
|---|---|---|
| 1 | Run `cosmic-order wallpaper add <image-url>` | Image downloaded and validated |
| 2 | Check `~/.local/share/backgrounds/custom/` | File exists with correct name |
| 3 | Try invalid URL | Error message, exit code 1 |
| 4 | Try non-image URL | "Invalid image" error, temp file cleaned up |

## Regression Checklist

Run before every release or after dependency upgrades (like libcosmic updates):

- [ ] `./scripts/health-check.sh` passes
- [ ] `just pre-commit` passes (fmt + clippy + tests)
- [ ] App launches and all three pages render
- [ ] Theme preview/confirm/cancel cycle works
- [ ] CLI `colors --json` produces valid JSON
- [ ] CLI `sync` completes without error
- [ ] At least one tool shows synced colors
- [ ] Screensaver "Save & Test" launches and dismisses
- [ ] No panics or crash output in terminal
