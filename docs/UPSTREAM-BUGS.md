# Upstream Bugs

Known bugs in upstream dependencies that affect COSMIC ORDER functionality.

## Active Issues

### Ghostty: `fullscreen=true` config ignored at startup on Wayland

- **Issue:** [ghostty-org/ghostty#11252](https://github.com/ghostty-org/ghostty/issues/11252)
  (auto-closed, vouching required)
- **Discussion:** [ghostty-org/ghostty#8579](https://github.com/ghostty-org/ghostty/discussions/8579)
- **Status:** Open / unresolved
- **Vendor:** Ghostty (Mitchell Hashimoto)
- **Affects:** Screensaver fullscreen launch via Ghostty

Ghostty's `fullscreen = true` and `maximize = true` config options are silently
ignored at startup on Linux/Wayland. The toggle keybind works fine once the window
is mapped. Root cause is an initialization timing issue — `present()` is called
before the WM processes the async `fullscreen()` request.

Confirmed across COSMIC, GNOME, KDE Plasma, Openbox, and Linux Mint.

**Impact on COSMIC ORDER:** The screensaver launch script cannot use
`ghostty --fullscreen=true` directly. Current workaround uses `wtype` to send
the fullscreen keybind after a short delay:

```bash
sleep 0.3
wtype -M ctrl -k Return -m ctrl
```

### cosmic-comp: Native COSMIC apps freeze on fullscreen

- **Issue:** [pop-os/cosmic-comp#2170](https://github.com/pop-os/cosmic-comp/issues/2170)
- **Original report:** [pop-os/cosmic-term#704](https://github.com/pop-os/cosmic-term/issues/704)
- **Status:** Open / unresolved
- **Vendor:** System76 (Pop!_OS / COSMIC team)
- **Affects:** All native COSMIC (iced/libcosmic) apps in fullscreen

Native COSMIC applications (cosmic-term, cosmic-edit, cosmic-files, cosmic-settings,
cosmic-store) freeze when entering fullscreen via Super+F11. The window becomes
completely unresponsive. Third-party GTK apps like Ghostty are not affected.

Confirmed on Pop!_OS 24.04 and CachyOS, across Intel and NVIDIA GPUs.

**Impact on COSMIC ORDER:** This is why COSMIC ORDER uses Ghostty instead of
cosmic-term as the terminal for screensaver rendering. If this is fixed, cosmic-term
could be reconsidered as a native alternative.
