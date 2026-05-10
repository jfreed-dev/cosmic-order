// SPDX-License-Identifier: GPL-3.0-only

//! Compositor settings management via cosmic-config
//!
//! Temporarily disables compositor features (autotile, focus-follows-cursor)
//! that interfere with the fullscreen screensaver test, then restores them.

use cosmic_config::{ConfigGet, ConfigSet};

const COMP_ID: &str = "com.system76.CosmicComp";
const COMP_VERSION: u64 = 1;

/// Saved compositor settings to restore after screensaver test
#[derive(Debug, Clone)]
pub struct CompositorBackup {
    autotile: Option<bool>,
    focus_follows_cursor: Option<bool>,
}

/// Read a bool key from compositor config, returning `None` on any error.
fn read_bool(config: &cosmic_config::Config, key: &str) -> Option<bool> {
    config.get(key).ok()
}

/// Disable compositor features that interfere with fullscreen screensaver.
///
/// Returns a backup of the original values, or `None` if the compositor
/// config is unavailable (non-fatal).
pub fn disable_interference() -> Result<Option<CompositorBackup>, String> {
    let config = match cosmic_config::Config::new(COMP_ID, COMP_VERSION) {
        Ok(c) => c,
        Err(e) => {
            tracing::warn!("Compositor config unavailable: {e}");
            return Ok(None);
        }
    };

    let autotile = read_bool(&config, "autotile");
    let focus_follows_cursor = read_bool(&config, "focus_follows_cursor");
    let needs_autotile = autotile == Some(true);
    let needs_focus = focus_follows_cursor == Some(true);

    if needs_autotile || needs_focus {
        let tx = config.transaction();
        if needs_autotile {
            tx.set("autotile", false)
                .map_err(|e| format!("Failed to disable autotile: {e}"))?;
        }
        if needs_focus {
            tx.set("focus_follows_cursor", false)
                .map_err(|e| format!("Failed to disable focus_follows_cursor: {e}"))?;
        }
        tx.commit()
            .map_err(|e| format!("Transaction commit failed: {e}"))?;
        tracing::info!(
            "Disabled compositor interference (autotile={needs_autotile}, focus={needs_focus})"
        );
    }

    Ok(Some(CompositorBackup {
        autotile,
        focus_follows_cursor,
    }))
}

/// Restore compositor settings from a previous backup.
pub fn restore_settings(backup: &CompositorBackup) -> Result<(), String> {
    let config = cosmic_config::Config::new(COMP_ID, COMP_VERSION)
        .map_err(|e| format!("Compositor config unavailable: {e}"))?;

    let tx = config.transaction();
    let mut any = false;

    if let Some(val) = backup.autotile {
        tx.set("autotile", val)
            .map_err(|e| format!("Failed to restore autotile: {e}"))?;
        any = true;
    }
    if let Some(val) = backup.focus_follows_cursor {
        tx.set("focus_follows_cursor", val)
            .map_err(|e| format!("Failed to restore focus_follows_cursor: {e}"))?;
        any = true;
    }

    if any {
        tx.commit()
            .map_err(|e| format!("Transaction commit failed: {e}"))?;
        tracing::info!(
            "Restored compositor settings (autotile={:?}, focus={:?})",
            backup.autotile,
            backup.focus_follows_cursor
        );
    }

    Ok(())
}
