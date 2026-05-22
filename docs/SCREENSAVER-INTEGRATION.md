# Screensaver Deep Integration Plan

Research findings and implementation roadmap for advanced screensaver features
in COSMIC ORDER.

## Executive Summary

Moving from shell scripts to a native COSMIC application enables deeper system
integration. This document outlines what's possible and breaks implementation
into achievable phases.

---

## 1. Power Management Integration

### What's Available

COSMIC uses **system76-power** and **UPower** via D-Bus:

| Service | D-Bus Path | Purpose |
|---------|------------|---------|
| UPower | `org.freedesktop.UPower` | Battery state, AC/DC detection |
| system76-power | `com.system76.PowerDaemon` | Power profiles |

### Power Profiles

Three profiles available via `com.system76.PowerDaemon`:

- `performance` - Maximum power, full effects
- `balanced` - Default mode
- `power-saver` - Battery mode, minimal effects

### Battery Detection

Via UPower DisplayDevice (`/org/freedesktop/UPower/devices/DisplayDevice`):

- `State` - Charging, Discharging, FullyCharged
- `Percentage` - 0-100%
- `OnBattery` - Boolean
- `TimeToEmpty` / `TimeToFull` - Seconds

### Implementation Approach

```toml
# Add to Cargo.toml
zbus = { version = "5.13", features = ["tokio"] }
upower_dbus = "0.3"
```

**Screensaver behavior by power state:**

| Condition | Behavior |
|-----------|----------|
| AC Power + Performance | Full effects (blackhole, matrix, etc.) |
| AC Power + Balanced | Standard effects |
| Battery + >50% | Simpler effects (rain, slide) |
| Battery + <20% | Minimal effects, faster timeout |
| Battery + <10% | Skip screensaver, let system sleep |

---

## 2. cosmic-term Integration

### Current Limitations

cosmic-term has **limited CLI support** compared to Ghostty:

| Feature | Ghostty | cosmic-term |
|---------|---------|-------------|
| `--fullscreen` flag | ✅ Yes | ❌ No |
| `--config-file` | ✅ Yes | ❌ No |
| `--class` window class | ✅ Yes | ❌ No |
| `-e` command execution | ✅ Yes | ✅ Yes |
| D-Bus control | ❌ No | ❌ Limited |
| Plugin system | ❌ No | ❌ No |

### Recommendation

**Use Alacritty as primary** for screensaver display: it self-fullscreens via
`startup_mode = "Fullscreen"`, sidestepping both the cosmic-term fullscreen
freeze and Ghostty's ignored-at-startup `fullscreen` (see
[UPSTREAM-BUGS.md](UPSTREAM-BUGS.md)). Ghostty and cosmic-term remain selectable.

**cosmic-term limitation:** no `--fullscreen`/`--class` flags, and it freezes
when fullscreened on COSMIC — so it can only run windowed.

### Future Possibilities

1. **Contribute to cosmic-term** - Add `--fullscreen` and `--class` flags
2. **Native rendering** - Render effects directly in COSMIC ORDER window
   (eliminates terminal dependency entirely)
3. **Wayland layer-shell** - Use layer-shell protocol for overlay surfaces

---

## 3. Cursor Hiding

### Current Implementation

Terminal-level cursor hiding works:

```bash
tput civis              # Hide text cursor
stty -echo              # Disable input echo
```

### Enhanced Approaches

| Method | Status | Notes |
|--------|--------|-------|
| `tput civis` | ✅ Working | Terminal text cursor only |
| Ghostty `mouse-hide-while-typing` | ✅ Working | Hides on typing |
| Fullscreen surface | 🔄 Test | COSMIC may auto-hide on fullscreen |
| Pointer constraints | 📋 Future | Confine pointer to surface |
| Session lock protocol | 📋 Future | Most robust solution |

### Wayland Constraints

- **wlrctl** doesn't work on COSMIC (requires wlroots, COSMIC uses Smithay)
- **xdotool** is X11-only
- **ydotool** works via libinput (potential option)

### Recommended Implementation

1. **Phase 1**: Verify current fullscreen behavior hides cursor
2. **Phase 2**: Add pointer confinement to screensaver surface
3. **Phase 3**: Implement session lock protocol if COSMIC supports it

---

## 4. Input Wake Detection

### Current Implementation

swayidle handles wake detection:

```bash
swayidle -w \
    timeout 300 'screensaver start' \
    resume 'screensaver stop'
```

### Enhancement Options

1. **D-Bus signal subscription** - Listen for input events
2. **libinput integration** - Direct input monitoring
3. **Compositor integration** - Use COSMIC's idle notification

### COSMIC Idle Protocol

COSMIC likely supports `ext-idle-notify-v1` Wayland protocol:

```rust
// Subscribe to idle state changes
ext_idle_notification_v1::get_idle_notification(
    seat,
    timeout_ms
)
```

---

## 5. Implementation Phases

### Phase A: Power-Aware Effects (Achievable)

**Goal**: Adjust screensaver behavior based on power state

**Tasks**:

1. Add `zbus` and `upower_dbus` dependencies
2. Create power monitoring service in COSMIC ORDER
3. Expose power state to screensaver configuration
4. Add effect profiles (performance, balanced, battery)
5. Update screensaver scripts to read power state

**Deliverables**:

- Power state displayed in UI
- Effect selection based on profile
- Battery-aware timeout adjustment

### Phase B: Enhanced Cursor Handling (Moderate)

**Goal**: Reliable cursor hiding during screensaver

**Tasks**:

1. Test current fullscreen cursor behavior on COSMIC
2. Document actual behavior
3. Implement pointer confinement if needed
4. Add cursor restore on wake

**Deliverables**:

- Cursor reliably hidden during screensaver
- Clean cursor restore on any input

### Phase C: Native Effect Rendering (Advanced)

**Goal**: Render effects directly in COSMIC ORDER window

**Tasks**:

1. Research iced/libcosmic animation capabilities
2. Port simple effects (fade, clock) to native rendering
3. Create effect plugin architecture
4. Implement fullscreen overlay window

**Deliverables**:

- Native clock effect
- Native fade transitions
- Framework for more effects

### Phase D: Compositor Integration (Advanced)

**Goal**: Deep integration with cosmic-comp

**Tasks**:

1. Implement session lock protocol
2. Use layer-shell for overlay surfaces
3. Direct idle notification subscription
4. Compositor-level cursor control

**Deliverables**:

- Lock screen integration
- Guaranteed cursor hiding
- Native idle detection

---

## 6. Dependencies to Add

```toml
# Power management
zbus = { version = "5.13", features = ["tokio"] }
upower_dbus = "0.3"

# Wayland protocols (for advanced features)
wayland-client = "0.31"
wayland-protocols = "0.31"

# Input handling (optional)
# libinput = "0.x"  # If direct input monitoring needed
```

---

## 7. Configuration Schema

Proposed screensaver configuration with power awareness:

```ron
(
    // Basic settings
    enabled: true,
    idle_timeout: 300,
    lock_timeout: 600,
    dpms_timeout: 900,

    // Power-aware profiles
    profiles: {
        "performance": (
            effects: ["blackhole", "matrix", "rain", "slide"],
            effect_duration: 30,
            clock_enabled: true,
            fade_enabled: true,
        ),
        "balanced": (
            effects: ["rain", "slide", "orbs"],
            effect_duration: 25,
            clock_enabled: true,
            fade_enabled: true,
        ),
        "battery": (
            effects: ["slide", "beams"],
            effect_duration: 20,
            clock_enabled: false,
            fade_enabled: false,
        ),
        "low_battery": (
            effects: ["slide"],
            effect_duration: 15,
            clock_enabled: false,
            fade_enabled: false,
        ),
    },

    // Thresholds
    low_battery_threshold: 20,
    critical_battery_threshold: 10,
    skip_on_critical: true,
)
```

---

## 8. Research Sources

- [system76-power](https://github.com/pop-os/system76-power)
- [cosmic-applets (battery)](https://github.com/pop-os/cosmic-applets)
- [cosmic-settings-daemon](https://github.com/pop-os/cosmic-settings-daemon)
- [UPower Reference](https://upower.freedesktop.org/docs/)
- [Wayland protocols](https://wayland.app/protocols/)
- [cosmic-term](https://github.com/pop-os/cosmic-term)
- [zbus documentation](https://docs.rs/zbus/)

---

## Summary

| Feature | Difficulty | Priority | Phase |
|---------|------------|----------|-------|
| Power profile detection | Easy | High | A |
| Battery state monitoring | Easy | High | A |
| Effect profile selection | Medium | High | A |
| Cursor hiding verification | Easy | Medium | B |
| Pointer confinement | Medium | Medium | B |
| Native effect rendering | Hard | Low | C |
| Session lock protocol | Hard | Low | D |
| cosmic-term fullscreen | Medium | Low | - |

The power-aware features (Phase A) provide the most value with moderate effort.
Start there and iterate based on user feedback.
