# Native Migration Plan

Architecture improvements to replace shell script dependencies with native
COSMIC/Linux APIs. Identified during deep codebase review (2026-02-08).

## Context

COSMIC ORDER currently relies on ~2,000 lines of shell scripts across
`screensaver-ctl.sh`, `launch-fullscreen.sh`, and `cosmic-screensaver.sh`
for screensaver lifecycle management. After NM-01/02/03, the Rust app has 1
remaining shell spawn point: `launch-fullscreen.sh` for Save & Test preview.

The standalone app architecture is confirmed correct — cosmic-settings has
no plugin system and System76 recommends standalone apps for third-party
functionality.

## Migration Items

### NM-01: Compositor Settings via cosmic-config

**Priority**: High | **Effort**: Low | **Status**: Complete

Replace `launch-fullscreen.sh` direct file writes to
`~/.config/cosmic/com.system76.CosmicComp/v1/` with cosmic-config API.

**Implementation**: `src/compositor.rs` uses `cosmic_config::Config` to
read/write `autotile` and `focus_follows_cursor` settings with backup/restore
pattern. Compositor picks up changes via inotify, no restart needed.

---

### NM-02: Read/Write cosmic-idle Config for DPMS

**Priority**: High | **Effort**: Low | **Status**: Complete

Read/write `CosmicIdleConfig` instead of maintaining a parallel
`DPMS_TIMEOUT` in custom shell config.

**Implementation**: `src/cosmic_idle.rs` reads `screen_off_time` from
`com.system76.CosmicIdle` config on startup to override local DPMS value.
On save, writes back so COSMIC Settings stays aligned. Values stored as
RON `Option<u32>` in milliseconds.

---

### NM-03: Systemd Service Management via D-Bus + Swayidle Config

**Priority**: Medium | **Effort**: Medium | **Status**: Complete

Replace `screensaver-ctl reload` shell-out with native swayidle config
generation and direct systemd D-Bus restart.

**Implementation**: `src/systemd.rs` provides `restart_user_unit()` using
`zbus::Connection::session()` to call `RestartUnit` on the systemd1 Manager
interface. `ScreensaverConfig::generate_swayidle_config()` writes
`~/.config/cosmic-screensaver/swayidle.conf` natively from config values.
Save flow: `config.save()` → `generate_swayidle_config()` → D-Bus restart →
cosmic-idle DPMS sync. Non-fatal on service restart failure (service may not
be running).

---

### NM-04: Screen Lock via logind D-Bus

**Priority**: Medium | **Effort**: Low | **Status**: Complete

Use `org.freedesktop.login1.Session.Lock()` for programmatic screen lock.

**Implementation**: `src/systemd.rs` provides `lock_session()` using
`zbus::Connection::system()` to call `Lock` on the logind Session interface
at `/org/freedesktop/login1/session/auto` (auto-resolves to caller's session).
Replaced `spawn_lock_command()` in `app.rs` with async `lock_screen()`.
Swayidle fallback updated to `loginctl lock-session`.

---

### NM-05: Event-Driven Power Monitoring

**Priority**: Medium | **Effort**: Medium | **Status**: Not started

Replace 5-second polling with D-Bus signal streams.

**Current**: `power.rs` polls UPower every 5 seconds via
`tokio::time::interval`.

**Native approach**:

```rust
let upower = UPowerProxy::new(&connection).await?;
let mut stream = upower.receive_on_battery_changed().await;
while let Some(change) = stream.next().await {
    let on_battery = change.get().await?;
    // emit update
}
```

Also subscribe to system76-power `PowerProfileSwitch` signal.

**Crate**: `upower_dbus` from `pop-os/dbus-settings-bindings` provides
typed proxy structs.

**Impact**: Lower CPU overhead, instant updates, cleaner code.

---

### NM-06: Screensaver Config to cosmic-config (Dual-Write)

**Priority**: Low | **Effort**: Medium | **Status**: Not started

Store canonical config via cosmic-config, generate shell format as derived
output.

**Current**: Custom `ScreensaverConfig` with shell KEY=value format at
`~/.config/cosmic-screensaver/config`. Required because shell scripts
source this file.

**Approach**: Keep shell format for backwards compatibility with
`screensaver-ctl.sh` and `cosmic-screensaver.sh`. Add a parallel
cosmic-config store so the Rust side uses proper RON/typed config. Generate
shell format on save.

**Prerequisite**: Only worth doing once NM-01 and NM-03 reduce shell
script reliance significantly.

---

### NM-07: Native Screensaver via Layer Shell

**Priority**: Low | **Effort**: Very High | **Status**: Research only

Replace terminal-based screensaver with a native layer-shell overlay surface.

**Current**: Alacritty (default) runs `cosmic-screensaver.sh` which uses
TerminalTextEffects (Python CLI) for animations. Alacritty self-fullscreens via
`startup_mode = "Fullscreen"`; ydotool is only used for optional mouse parking.

**Native approach**: Use `zwlr-layer-shell-v1` to create an overlay
surface covering all outputs. Render effects directly via iced/wgpu.
Alternatively, use `ext-session-lock-v1` for proper session lock
integration.

**Blockers**:

- TerminalTextEffects is Python — would need Rust port or wgpu renderer
- cosmic-idle already uses layer shell for fade-to-black
- Would eliminate ~2,000 lines of shell scripts entirely

**Impact**: Transformative but very high effort. Tracked under the Phase 7
"Deep Compositor Integration" future track in [ROADMAP.md](../ROADMAP.md)
(Phase 7A/7B already shipped in v0.12.0).

---

### NM-08: Idle Inhibitor via Wayland Protocol

**Status**: Obsolete — won't do

The caffeine / idle-inhibitor feature was removed; idle inhibition is left to
external tools (`systemd-inhibit` or a COSMIC applet), so no native migration is
needed.

---

## Build & Tooling Improvements

### BT-01: Justfile Conventions

**Priority**: High | **Effort**: Low | **Status**: Complete

Align justfile with cosmic-app-template conventions: variables for paths,
vendor support, proper install destinations with rootdir/prefix.

---

### BT-02: AppStream Metadata

**Priority**: Medium | **Effort**: Low | **Status**: Not started

Create `com.github.jfreed-dev.CosmicOrder.metainfo.xml` for COSMIC Store
listing. Add "COSMIC" to categories for "Made for COSMIC" section.

---

## Decision Log

| Date | Decision | Rationale |
|------|----------|-----------|
| 2026-02-08 | Stay standalone (not cosmic-settings) | No plugin system; all tools standalone |
| 2026-02-08 | Keep shell config format (for now) | Scripts must source it; migrate after NM-01/03 |
| 2026-02-08 | Defer layer-shell screensaver to Phase 7 future track | Very high effort; current approach works |
| 2026-02-08 | NM-01/02/03 complete | Compositor, DPMS, service mgmt native; shell launch remains |
