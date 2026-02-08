# Native Migration Plan

Architecture improvements to replace shell script dependencies with native
COSMIC/Linux APIs. Identified during deep codebase review (2026-02-08).

## Context

COSMIC ORDER currently relies on ~2,000 lines of shell scripts across
`screensaver-ctl.sh`, `launch-fullscreen.sh`, and `cosmic-screensaver.sh`
for screensaver lifecycle management. The Rust app has 2 direct shell spawn
points (`app.rs:661-667`, `app.rs:678-695`).

The standalone app architecture is confirmed correct — cosmic-settings has
no plugin system and System76 recommends standalone apps for third-party
functionality.

## Migration Items

### NM-01: Compositor Settings via cosmic-config

**Priority**: High | **Effort**: Low | **Status**: Not started

Replace `launch-fullscreen.sh` direct file writes to
`~/.config/cosmic/com.system76.CosmicComp/v1/` with cosmic-config API.

**Current**: Shell script writes `autotile` and `focus_follows_cursor` files
directly, relies on inotify for compositor reload.

**Native approach**:

```rust
let comp = cosmic_config::Config::new("com.system76.CosmicComp", 1)?;
let saved_autotile: bool = comp.get("autotile")?;
let saved_ffc: bool = comp.get("focus_follows_cursor")?;

let tx = comp.transaction();
tx.set("autotile", false)?;
tx.set("focus_follows_cursor", false)?;
tx.commit()?;
// ... screensaver runs, then restore ...
```

**Impact**: Eliminates the most fragile shell interaction. Same inotify
mechanism, but through the proper API with error handling and type safety.

---

### NM-02: Read/Write cosmic-idle Config for DPMS

**Priority**: High | **Effort**: Low | **Status**: Not started

Read/write `CosmicIdleConfig` instead of maintaining a parallel
`DPMS_TIMEOUT` in custom shell config.

**Current**: App stores `DPMS_TIMEOUT` in `~/.config/cosmic-screensaver/config`
(shell KEY=value format). Separate from system idle settings.

**Native approach**:

```rust
let idle_config = cosmic_config::Config::new("com.system76.CosmicIdle", 1)?;
let screen_off: Option<u32> = idle_config.get("screen_off_time")?;
// Values in milliseconds, stored as RON Option<u32>
```

**Fields available**: `screen_off_time`, `suspend_on_battery_time`,
`suspend_on_ac_time`.

**Impact**: Aligns DPMS settings with what cosmic-settings shows. Users
see consistent values across both apps.

---

### NM-03: Systemd Service Management via D-Bus

**Priority**: Medium | **Effort**: Medium | **Status**: Not started

Replace `screensaver-ctl reload` shell-out with direct systemd D-Bus call.

**Current**: `app.rs:688-691` spawns `screensaver-ctl reload` which calls
`systemctl --user restart cosmic-screensaver-idle.service`.

**Native approach**:

```rust
// org.freedesktop.systemd1.Manager.RestartUnit()
let manager = zbus::proxy::ManagerProxy::new(&connection).await?;
manager.restart_unit("cosmic-screensaver-idle.service", "replace").await?;
```

**Impact**: Removes one of the two shell spawn points. Still needs
screensaver-ctl for swayidle config generation unless that is also
ported (higher effort).

---

### NM-04: Screen Lock via logind D-Bus

**Priority**: Medium | **Effort**: Low | **Status**: Not started

Use `org.freedesktop.login1.Session.Lock()` for programmatic screen lock.

**Current**: Shell scripts call `loginctl lock-session` or
`cosmic-greeter lock` as subprocesses.

**Native approach**:

```rust
let session = zbus::proxy::SessionProxy::new(&connection).await?;
session.lock().await?;
```

**Impact**: Direct D-Bus call instead of spawning a subprocess.

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

**Current**: Ghostty terminal runs `cosmic-screensaver.sh` which uses
TerminalTextEffects (Python CLI) for animations. Requires ydotool for
fullscreen, compositor setting manipulation, mouse parking, etc.

**Native approach**: Use `zwlr-layer-shell-v1` to create an overlay
surface covering all outputs. Render effects directly via iced/wgpu.
Alternatively, use `ext-session-lock-v1` for proper session lock
integration.

**Blockers**:
- TerminalTextEffects is Python — would need Rust port or wgpu renderer
- cosmic-idle already uses layer shell for fade-to-black
- Would eliminate ~2,000 lines of shell scripts entirely

**Impact**: Transformative but very high effort. Consider for Phase 7.

---

### NM-08: Idle Inhibitor via Wayland Protocol

**Priority**: Medium | **Effort**: Low | **Status**: Not started
**Phase**: 4C (Caffeine Mode)

Use `zwp-idle-inhibit-unstable-v1` or `ext-idle-notify-v1` Wayland
protocol for idle inhibition instead of shell-based approaches.

**Also available**: `org.freedesktop.login1.Manager.Inhibit()` D-Bus call
returns a file descriptor — holding it open prevents idle actions.

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
| 2026-02-08 | Stay standalone (not cosmic-settings) | No plugin system exists; all community tools are standalone |
| 2026-02-08 | Keep shell config format (for now) | Scripts must source it; migrate after NM-01/NM-03 |
| 2026-02-08 | Defer layer-shell screensaver to Phase 7 | Very high effort; current approach works |
