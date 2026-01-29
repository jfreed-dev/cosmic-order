# PR Review Log

Tracks internal code review suggestions and their resolution.

| PR | Title | Status | Resolution |
|----|-------|--------|------------|
| 01 | Fix app ID mismatch for config persistence | Implemented | Unified `config.rs` to use `crate::APP_ID` instead of hardcoded `com.system76.CosmicOrder` |
| 02 | Apply theme presets and imports fully | Deferred | Theme preview only toggles dark/light mode; full palette application requires deeper `ThemeBuilder` integration — revisit in Phase 5 polish |
| 03 | Localize theme labels on Themes page | Implemented | Replaced hardcoded "Dark"/"Light" with `fl!("theme-mode-dark")` / `fl!("theme-mode-light")` |
