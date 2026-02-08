// SPDX-License-Identifier: GPL-3.0-only

//! Systemd service management via D-Bus
//!
//! Provides user-level systemd unit control through the session bus.

/// Restart a systemd user unit via D-Bus
pub async fn restart_user_unit(unit: &str) -> Result<(), String> {
    let connection = zbus::Connection::session()
        .await
        .map_err(|e| format!("D-Bus session connection failed: {e}"))?;

    connection
        .call_method(
            Some("org.freedesktop.systemd1"),
            "/org/freedesktop/systemd1",
            Some("org.freedesktop.systemd1.Manager"),
            "RestartUnit",
            &(unit, "replace"),
        )
        .await
        .map_err(|e| {
            tracing::warn!("Failed to restart {unit}: {e}");
            format!("RestartUnit failed: {e}")
        })?;

    tracing::info!("Restarted systemd unit: {unit}");
    Ok(())
}
