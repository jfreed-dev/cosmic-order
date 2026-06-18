# Upstream Bugs

Known bugs in upstream dependencies that affect COSMIC ORDER functionality.

_Last verified 2026-06-18 against COSMIC Epoch 1.0.16._

## Active Issues

### Ghostty: `fullscreen=true` config ignored at startup on Wayland

- **Issue:** [ghostty-org/ghostty#11252](https://github.com/ghostty-org/ghostty/issues/11252)
- **Discussion:** [ghostty-org/ghostty#8579](https://github.com/ghostty-org/ghostty/discussions/8579)
- **Status:** Closed as `not planned` (2026-03-09) under Ghostty's vouching
  policy — effectively won't-fix
- **Vendor:** Ghostty (Mitchell Hashimoto)
- **Affects:** Screensaver fullscreen launch via Ghostty

Ghostty's `fullscreen = true` and `maximize = true` config options are silently
ignored at startup on Linux/Wayland. The toggle keybind works fine once the window
is mapped. Root cause is an initialization timing issue — `present()` is called
before the WM processes the async `fullscreen()` request.

Confirmed across COSMIC, GNOME, KDE Plasma, Openbox, and Linux Mint.

**Impact on COSMIC ORDER:** Affects Ghostty only as a *selectable* screensaver
terminal — the default is now Alacritty, which self-fullscreens via
`startup_mode = "Fullscreen"` and avoids this bug entirely. When Ghostty is
chosen, the screensaver toggles fullscreen via the compositor keybind after
launch. Since upstream has declined to fix it, this keybind toggle is the
permanent path for Ghostty — not a temporary workaround.

### cosmic-comp: Native COSMIC apps freeze on fullscreen

- **Issue:** [pop-os/cosmic-comp#2170](https://github.com/pop-os/cosmic-comp/issues/2170)
- **Original report:** [pop-os/cosmic-term#704](https://github.com/pop-os/cosmic-term/issues/704)
- **Status:** Open / unresolved (still open as of COSMIC Epoch 1.0.16, 2026-06-10)
- **Vendor:** System76 (Pop!_OS / COSMIC team)
- **Affects:** All native COSMIC (iced/libcosmic) apps in fullscreen

Native COSMIC applications (cosmic-term, cosmic-edit, cosmic-files, cosmic-settings,
cosmic-store) freeze when entering fullscreen via Super+F11. The window becomes
completely unresponsive. Third-party GTK apps like Ghostty are not affected.

Confirmed on Pop!_OS 24.04 and CachyOS, across Intel and NVIDIA GPUs.

Root cause narrowed (issue thread, May 2026): a futex/mutex deadlock in the
XDG-activation-token → surface-mapping handshake. The freeze only reproduces
when the app is launched through `cosmic-launcher`'s `systemd-run --scope` (with
`XDG_ACTIVATION_TOKEN` / `DESKTOP_STARTUP_ID` set); launching the same binary
via `setsid` does not freeze. Not GPU/Vulkan-specific — it still reproduces with
`WGPU_BACKEND=gl`.

**Impact on COSMIC ORDER:** This is why the screensaver defaults to Alacritty
rather than cosmic-term. If this is fixed, cosmic-term could be reconsidered as a
native alternative.
