// SPDX-License-Identifier: GPL-3.0-only

//! User-defined hook system for theme sync
//!
//! Runs executable scripts in `~/.config/cosmic-order/hooks.d/` after
//! built-in generators complete. Each hook receives palette env vars
//! and the path to `colors.toml` as its first argument.

use std::path::{Path, PathBuf};

use crate::colors::ColorPalette;
use crate::paths;

/// Results from running user hooks
#[derive(Debug, Clone, Default)]
#[allow(clippy::struct_field_names)] // Fields mirror the domain concept "hooks"
pub struct HookResults {
    pub hooks_run: u32,
    pub hooks_succeeded: u32,
    pub hooks_failed: u32,
    pub hooks_timed_out: u32,
}

/// Ensure the hooks directory exists, returning its path
pub async fn ensure_hooks_dir() -> Result<PathBuf, std::io::Error> {
    let dir = hooks_dir();
    tokio::fs::create_dir_all(&dir).await?;
    Ok(dir)
}

/// Run all executable hooks in `hooks.d/` with palette env vars
#[allow(clippy::cognitive_complexity)]
pub async fn run_hooks(palette: &ColorPalette, colors_path: &Path) -> HookResults {
    let dir = match ensure_hooks_dir().await {
        Ok(d) => d,
        Err(e) => {
            tracing::warn!("Failed to ensure hooks directory: {e}");
            return HookResults::default();
        }
    };

    let mut entries: Vec<PathBuf> = Vec::new();
    let mut read_dir = match tokio::fs::read_dir(&dir).await {
        Ok(rd) => rd,
        Err(e) => {
            tracing::warn!("Failed to read hooks directory: {e}");
            return HookResults::default();
        }
    };

    while let Ok(Some(entry)) = read_dir.next_entry().await {
        let path = entry.path();
        if path.is_file() {
            entries.push(path);
        }
    }

    entries.sort();

    let mut results = HookResults::default();
    let envs = build_env_vars(palette, colors_path);

    for hook_path in &entries {
        // Check if executable
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            if let Ok(meta) = tokio::fs::metadata(hook_path).await
                && meta.permissions().mode() & 0o111 == 0
            {
                tracing::debug!("Skipping non-executable hook: {}", hook_path.display());
                continue;
            }
        }

        results.hooks_run += 1;
        let hook_name = hook_path.file_name().map_or_else(
            || "unknown".to_string(),
            |n| n.to_string_lossy().to_string(),
        );

        tracing::info!("Running hook: {hook_name}");

        let mut cmd = tokio::process::Command::new(hook_path);
        cmd.arg(colors_path);
        for (key, val) in &envs {
            cmd.env(key, val);
        }

        let result = tokio::time::timeout(std::time::Duration::from_secs(10), cmd.output()).await;

        match result {
            Ok(Ok(output)) => {
                if output.status.success() {
                    tracing::info!("Hook succeeded: {hook_name}");
                    results.hooks_succeeded += 1;
                } else {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    tracing::warn!(
                        "Hook failed: {hook_name} (exit {}): {stderr}",
                        output.status
                    );
                    results.hooks_failed += 1;
                }
            }
            Ok(Err(e)) => {
                tracing::warn!("Hook error: {hook_name}: {e}");
                results.hooks_failed += 1;
            }
            Err(_) => {
                tracing::warn!("Hook timed out (10s): {hook_name}");
                results.hooks_timed_out += 1;
            }
        }
    }

    results
}

/// Build environment variables from palette for hooks
fn build_env_vars(palette: &ColorPalette, colors_path: &Path) -> Vec<(String, String)> {
    let mut vars = Vec::with_capacity(24);
    vars.push(("COSMIC_BG".to_string(), palette.background.clone()));
    vars.push(("COSMIC_FG".to_string(), palette.foreground.clone()));
    vars.push(("COSMIC_ACCENT".to_string(), palette.accent.clone()));
    vars.push(("COSMIC_CURSOR".to_string(), palette.cursor.clone()));
    vars.push((
        "COSMIC_SEL_FG".to_string(),
        palette.selection_foreground.clone(),
    ));
    vars.push((
        "COSMIC_SEL_BG".to_string(),
        palette.selection_background.clone(),
    ));

    for (i, color) in palette.colors.iter().enumerate() {
        vars.push((format!("COSMIC_COLOR{i}"), color.clone()));
    }

    vars.push((
        "COSMIC_COLORS_PATH".to_string(),
        colors_path.display().to_string(),
    ));

    vars
}

fn hooks_dir() -> PathBuf {
    paths::hooks_dir()
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn test_build_env_vars_count() {
        let palette = ColorPalette::sample();
        let path = PathBuf::from("/tmp/colors.toml");
        let vars = build_env_vars(&palette, &path);

        // 6 named + 16 colors + 1 path = 23
        assert_eq!(vars.len(), 23);
    }

    #[test]
    fn test_build_env_vars_content() {
        let palette = ColorPalette::sample();
        let path = PathBuf::from("/tmp/colors.toml");
        let vars = build_env_vars(&palette, &path);

        let find = |key: &str| vars.iter().find(|(k, _)| k == key).map(|(_, v)| v.as_str());

        assert_eq!(find("COSMIC_BG"), Some("#1B1B1B"));
        assert_eq!(find("COSMIC_FG"), Some("#FFFFFF"));
        assert_eq!(find("COSMIC_ACCENT"), Some("#63D1D6"));
        assert_eq!(find("COSMIC_CURSOR"), Some("#FFFFFF"));
        assert_eq!(find("COSMIC_SEL_FG"), Some("#1B1B1B"));
        assert_eq!(find("COSMIC_SEL_BG"), Some("#FFFFFF"));
        assert_eq!(find("COSMIC_COLOR0"), Some("#3B3B3B"));
        assert_eq!(find("COSMIC_COLOR15"), Some("#E8E8E8"));
        assert_eq!(find("COSMIC_COLORS_PATH"), Some("/tmp/colors.toml"));
    }

    #[test]
    fn test_hooks_dir_path() {
        let dir = hooks_dir();
        assert!(dir.to_string_lossy().contains("hooks.d"));
    }

    #[test]
    fn test_hook_results_default() {
        let results = HookResults::default();
        assert_eq!(results.hooks_run, 0);
        assert_eq!(results.hooks_succeeded, 0);
        assert_eq!(results.hooks_failed, 0);
        assert_eq!(results.hooks_timed_out, 0);
    }
}
