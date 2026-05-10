// SPDX-License-Identifier: GPL-3.0-only

//! Systemd and logind D-Bus integration
//!
//! Provides user-level systemd unit control through the session bus
//! and session lock via logind on the system bus.

/// Stop a systemd user unit via D-Bus
pub async fn stop_user_unit(unit: &str) -> Result<(), String> {
    let connection = zbus::Connection::session()
        .await
        .map_err(|e| format!("D-Bus session connection failed: {e}"))?;

    connection
        .call_method(
            Some("org.freedesktop.systemd1"),
            "/org/freedesktop/systemd1",
            Some("org.freedesktop.systemd1.Manager"),
            "StopUnit",
            &(unit, "replace"),
        )
        .await
        .map_err(|e| {
            tracing::warn!("Failed to stop {unit}: {e}");
            format!("StopUnit failed: {e}")
        })?;

    tracing::info!("Stopped systemd unit: {unit}");
    Ok(())
}

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

/// Lock the current session via logind D-Bus
pub async fn lock_session() -> Result<(), String> {
    let connection = zbus::Connection::system()
        .await
        .map_err(|e| format!("D-Bus system connection failed: {e}"))?;

    connection
        .call_method(
            Some("org.freedesktop.login1"),
            "/org/freedesktop/login1/session/auto",
            Some("org.freedesktop.login1.Session"),
            "Lock",
            &(),
        )
        .await
        .map_err(|e| {
            tracing::warn!("Failed to lock session: {e}");
            format!("Session Lock failed: {e}")
        })?;

    tracing::info!("Locked session via logind");
    Ok(())
}
