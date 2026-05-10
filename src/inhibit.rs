// SPDX-License-Identifier: GPL-3.0-only

//! Idle inhibitor via logind D-Bus API
//!
//! Calls `org.freedesktop.login1.Manager.Inhibit("idle", ...)` which returns
//! a file descriptor. Holding it open blocks system idle; dropping it restores
//! normal behavior.

use std::sync::Arc;

/// Holds a logind idle inhibitor lock.
///
/// The inhibitor is active as long as this struct exists. Dropping it
/// closes the file descriptor, which releases the lock.
///
/// Wrapped in `Arc` so it can be passed through libcosmic's `Clone`-requiring
/// message enum without duplicating the fd.
#[derive(Debug, Clone)]
pub struct IdleInhibitor {
    _fd: Arc<zbus::zvariant::OwnedFd>,
}

impl IdleInhibitor {
    /// Acquire an idle inhibitor from logind via the system D-Bus.
    pub async fn acquire() -> Result<Self, String> {
        let connection = zbus::Connection::system()
            .await
            .map_err(|e| format!("D-Bus system connection failed: {e}"))?;

        let reply = connection
            .call_method(
                Some("org.freedesktop.login1"),
                "/org/freedesktop/login1",
                Some("org.freedesktop.login1.Manager"),
                "Inhibit",
                &("idle", "COSMIC ORDER", "Caffeine mode", "block"),
            )
            .await
            .map_err(|e| format!("Inhibit call failed: {e}"))?;

        let fd: zbus::zvariant::OwnedFd = reply
            .body()
            .deserialize()
            .map_err(|e| format!("Failed to deserialize inhibitor fd: {e}"))?;

        tracing::info!("Idle inhibitor acquired");
        Ok(Self { _fd: Arc::new(fd) })
    }
}
