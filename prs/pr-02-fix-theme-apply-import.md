# PR 02: Apply theme presets and imports fully

Issue

- Theme "preview/apply" only toggles dark/light mode and does
  not apply the preset's palette or high-contrast variants.
- Theme import writes the theme file but does not activate it
  when the imported theme's mode differs from the current mode.
- This makes "Try" feel like a no-op beyond dark/light and
  causes imported themes to appear ineffective.

Remediation plan / justification

- Ensure preview/apply uses the full `CosmicTheme` object
  (including palette/accent/high-contrast) rather than just
  `ThemeMode`.
- Ensure imports also switch the theme mode to match the
  imported theme so the new theme becomes active immediately.
- Aligns behavior with Phase 2 expectations (theme preview and
  import/export).

Suggested resolution

- Extend `ThemePreview::apply` to write the full theme to the
  correct config and set ThemeMode to match.
- On import, write the imported theme and update ThemeMode based
  on `imported.is_dark`.
- Update UI state after apply/import to reflect the active theme
  name and mode.
