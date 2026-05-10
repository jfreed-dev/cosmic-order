// SPDX-License-Identifier: GPL-3.0-only

//! Read/write COSMIC idle config (`com.system76.CosmicIdle`)
//!
//! Syncs the DPMS (screen-off) timeout with the system-wide setting
//! so that cosmic-order and COSMIC Settings stay aligned.

use cosmic_config::{ConfigGet, ConfigSet};

const IDLE_ID: &str = "com.system76.CosmicIdle";
const IDLE_VERSION: u64 = 1;

/// Read `screen_off_time` from system config, converting ms → seconds.
///
/// Returns `Some(seconds)` on success (`None` RON value → 0 seconds),
/// or `None` if the config is unavailable.
pub fn read_screen_off_time() -> Option<u32> {
    let config = match cosmic_config::Config::new(IDLE_ID, IDLE_VERSION) {
        Ok(c) => c,
        Err(e) => {
            tracing::warn!("CosmicIdle config unavailable: {e}");
            return None;
        }
    };

    match config.get::<Option<u32>>("screen_off_time") {
        Ok(Some(ms)) => Some(ms / 1000),
        Ok(None) => Some(0),
        Err(e) => {
            tracing::warn!("Failed to read screen_off_time: {e}");
            None
        }
    }
}

/// Write `screen_off_time` to system config, converting seconds → ms.
///
/// 0 seconds maps to `None` (disabled). Logs on failure, never fatal.
pub fn write_screen_off_time(seconds: u32) {
    let config = match cosmic_config::Config::new(IDLE_ID, IDLE_VERSION) {
        Ok(c) => c,
        Err(e) => {
            tracing::warn!("CosmicIdle config unavailable for write: {e}");
            return;
        }
    };

    let value: Option<u32> = if seconds == 0 {
        None
    } else {
        Some(seconds * 1000)
    };

    if let Err(e) = config.set("screen_off_time", value) {
        tracing::warn!("Failed to write screen_off_time: {e}");
    }
}
