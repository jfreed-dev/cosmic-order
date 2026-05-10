// SPDX-License-Identifier: GPL-3.0-only

#[allow(clippy::wildcard_imports)]
use super::*;

impl App {
    /// Handle Wayland idle events
    #[allow(clippy::cognitive_complexity)]
    pub(super) fn handle_idle_event(&mut self, event: wayland_idle::IdleEvent) -> Task<Message> {
        match event {
            wayland_idle::IdleEvent::Connected => {
                tracing::info!("Native idle detection connected — stopping swayidle");
                self.native_idle_active = true;
                cosmic::task::future(async {
                    if let Err(e) =
                        crate::systemd::stop_user_unit("cosmic-screensaver-idle.service").await
                    {
                        tracing::warn!("Failed to stop swayidle (may not be running): {e}");
                    }
                    Message::None
                })
            }
            wayland_idle::IdleEvent::ScreensaverIdle => {
                // Respect caffeine mode
                if self.caffeine_active {
                    tracing::debug!("Screensaver idle ignored — caffeine mode active");
                    return Task::none();
                }

                tracing::info!("Screensaver idle — launching screensaver");
                let launcher = ScreensaverConfig::fullscreen_launcher_path();
                if !launcher.exists() {
                    tracing::warn!("launch-fullscreen.sh not found at {}", launcher.display());
                    return Task::none();
                }

                match std::process::Command::new(&launcher)
                    .arg("launch")
                    .arg("force")
                    .arg("--skip-compositor")
                    .spawn()
                {
                    Ok(child) => {
                        self.idle_screensaver_child = Some(child.id());
                        tracing::info!("Screensaver launched (pid {})", child.id());
                    }
                    Err(e) => {
                        tracing::error!("Failed to launch screensaver: {e}");
                    }
                }

                // Schedule lock timer if session lock is enabled
                let lock_timeout = self.screensaver_config.lock_timeout;
                if self.session_lock_enabled && lock_timeout > 0 && self.lock_timer_handle.is_none()
                {
                    tracing::info!("Scheduling lock in {lock_timeout}s");
                    let (task, handle) = Task::abortable(cosmic::task::future(async move {
                        tokio::time::sleep(std::time::Duration::from_secs(u64::from(lock_timeout)))
                            .await;
                        Message::LockScreen
                    }));
                    self.lock_timer_handle = Some(handle);
                    return task;
                }
                Task::none()
            }
            wayland_idle::IdleEvent::ScreensaverResumed => {
                tracing::info!("User activity resumed — killing screensaver");
                self.kill_idle_screensaver();
                // Cancel pending lock timer
                if let Some(handle) = self.lock_timer_handle.take() {
                    handle.abort();
                    tracing::debug!("Lock timer cancelled");
                }
                Task::none()
            }
            wayland_idle::IdleEvent::LockIdle => {
                // Fallback: Wayland lock notification (may not fire reliably
                // when screensaver window resets idle timer)
                tracing::info!("Lock idle — locking screen");
                self.lock_screen()
            }
            wayland_idle::IdleEvent::Error(e) => {
                tracing::warn!("Idle subscription error: {e} — falling back to swayidle");
                self.native_idle_active = false;
                cosmic::task::future(async {
                    if let Err(e) =
                        crate::systemd::restart_user_unit("cosmic-screensaver-idle.service").await
                    {
                        tracing::warn!("Failed to restart swayidle: {e}");
                    }
                    Message::None
                })
            }
        }
    }

    /// Handle logind sleep events
    #[allow(clippy::needless_pass_by_value)] // Elm architecture message pattern
    pub(super) fn handle_sleep_event(&mut self, event: sleep_lock::SleepEvent) -> Task<Message> {
        match event {
            sleep_lock::SleepEvent::PrepareForSleep => {
                tracing::info!("System going to sleep — locking screen");
                self.lock_screen()
            }
        }
    }

    /// Kill the screensaver child process if running
    pub(super) fn kill_idle_screensaver(&mut self) {
        if self.idle_screensaver_child.take().is_some() {
            let launcher = ScreensaverConfig::fullscreen_launcher_path();
            if let Err(e) = std::process::Command::new(&launcher).arg("kill").status() {
                tracing::warn!("Failed to kill screensaver via launcher: {e}");
            }
        }
    }

    /// Lock the screen via logind D-Bus (triggers COSMIC greeter)
    ///
    /// Note: In-process ext-session-lock-v1 is not viable because acquiring
    /// the lock disrupts the main app's Wayland connection (broken pipe),
    /// crashing the app while the lock is held. A separate binary would be
    /// needed for native session lock; for now we use loginctl lock-session.
    #[allow(clippy::unused_self)] // Method pattern; may use self in future lock strategies
    pub(super) fn lock_screen(&mut self) -> Task<Message> {
        tracing::info!("Locking screen via logind D-Bus");
        cosmic::task::future(async {
            if let Err(e) = crate::systemd::lock_session().await {
                tracing::error!("Failed to lock screen: {e}");
            }
            Message::None
        })
    }

    /// Synchronously restart swayidle — used during app exit when tokio may be shutting down
    pub(super) fn restart_swayidle_sync() {
        match std::process::Command::new("systemctl")
            .args(["--user", "restart", "cosmic-screensaver-idle.service"])
            .status()
        {
            Ok(status) => {
                if status.success() {
                    tracing::info!("Swayidle service restarted on app exit");
                } else {
                    tracing::warn!("Swayidle restart exited with: {status}");
                }
            }
            Err(e) => tracing::warn!("Failed to restart swayidle on exit: {e}"),
        }
    }

    /// Compute idle subscription config from screensaver settings
    pub(super) const fn compute_idle_config(
        config: &ScreensaverConfig,
    ) -> wayland_idle::IdleSubscriptionConfig {
        let screensaver_timeout_ms = if config.enabled {
            config.idle_timeout.saturating_mul(1000)
        } else {
            0
        };
        let lock_timeout_ms = if config.enabled && config.lock_timeout > 0 {
            (config.idle_timeout + config.lock_timeout).saturating_mul(1000)
        } else {
            0
        };
        wayland_idle::IdleSubscriptionConfig {
            screensaver_timeout_ms,
            lock_timeout_ms,
            enabled: config.enabled,
        }
    }
}
