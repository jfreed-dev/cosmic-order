// SPDX-License-Identifier: GPL-3.0-only

//! Native Wayland idle detection via ext-idle-notify-v1
//!
//! Subscribes to the compositor's idle notification protocol for screensaver
//! and lock timeouts. Falls back to swayidle when the protocol is unavailable.

use std::any::TypeId;
use std::sync::mpsc as std_mpsc;

use wayland_client::protocol::{wl_registry, wl_seat};
use wayland_client::{Connection, Dispatch, QueueHandle, delegate_noop};
use wayland_protocols::ext::idle_notify::v1::client::{
    ext_idle_notification_v1::{self, ExtIdleNotificationV1},
    ext_idle_notifier_v1::ExtIdleNotifierV1,
};

/// Configuration for the idle subscription. Changing any field causes iced to
/// restart the subscription automatically (because the ID changes).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct IdleSubscriptionConfig {
    /// Screensaver idle timeout in milliseconds (0 = disabled)
    pub screensaver_timeout_ms: u32,
    /// Lock timeout in milliseconds (0 = disabled)
    pub lock_timeout_ms: u32,
    /// Master enable switch
    pub enabled: bool,
}

/// Events emitted by the idle subscription
#[derive(Debug, Clone)]
pub enum IdleEvent {
    /// Screensaver idle timeout reached
    ScreensaverIdle,
    /// User activity resumed after screensaver idle
    ScreensaverResumed,
    /// Lock timeout reached
    LockIdle,
    /// Successfully connected to compositor idle protocol
    Connected,
    /// Protocol unavailable or connection error
    Error(String),
}

/// User data tag to distinguish screensaver vs lock notifications
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum NotificationKind {
    Screensaver,
    Lock,
}

/// Wayland dispatch state
struct IdleState {
    /// Channel to send events back to the iced subscription
    tx: std_mpsc::Sender<IdleEvent>,
    /// Bound notifier global (None until registry bind)
    notifier: Option<ExtIdleNotifierV1>,
    /// Bound seat (None until registry bind)
    seat: Option<wl_seat::WlSeat>,
}

// --- Dispatch implementations ---

impl Dispatch<wl_registry::WlRegistry, ()> for IdleState {
    fn event(
        state: &mut Self,
        registry: &wl_registry::WlRegistry,
        event: wl_registry::Event,
        _data: &(),
        _conn: &Connection,
        qh: &QueueHandle<Self>,
    ) {
        if let wl_registry::Event::Global {
            name,
            interface,
            version,
        } = event
        {
            match interface.as_str() {
                "ext_idle_notifier_v1" => {
                    let notifier =
                        registry.bind::<ExtIdleNotifierV1, _, Self>(name, version, qh, ());
                    state.notifier = Some(notifier);
                }
                "wl_seat" => {
                    // Bind only the first seat
                    if state.seat.is_none() {
                        let seat = registry.bind::<wl_seat::WlSeat, _, Self>(name, version, qh, ());
                        state.seat = Some(seat);
                    }
                }
                _ => {}
            }
        }
    }
}

delegate_noop!(IdleState: ignore wl_seat::WlSeat);
delegate_noop!(IdleState: ignore ExtIdleNotifierV1);

impl Dispatch<ExtIdleNotificationV1, NotificationKind> for IdleState {
    fn event(
        state: &mut Self,
        _proxy: &ExtIdleNotificationV1,
        event: ext_idle_notification_v1::Event,
        kind: &NotificationKind,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        let idle_event = match (event, kind) {
            (ext_idle_notification_v1::Event::Idled, NotificationKind::Screensaver) => {
                IdleEvent::ScreensaverIdle
            }
            (ext_idle_notification_v1::Event::Resumed, NotificationKind::Screensaver) => {
                IdleEvent::ScreensaverResumed
            }
            (ext_idle_notification_v1::Event::Idled, NotificationKind::Lock) => IdleEvent::LockIdle,
            (ext_idle_notification_v1::Event::Resumed, NotificationKind::Lock) => {
                // Lock resumed — no action needed
                return;
            }
            _ => return,
        };
        let _ = state.tx.send(idle_event);
    }
}

/// Create a libcosmic subscription for Wayland idle notifications.
///
/// Uses `(TypeId, config)` as the subscription ID so that config changes
/// automatically restart the subscription with updated timeouts.
pub fn idle_subscription(config: IdleSubscriptionConfig) -> cosmic::iced::Subscription<IdleEvent> {
    if !config.enabled || (config.screensaver_timeout_ms == 0 && config.lock_timeout_ms == 0) {
        return cosmic::iced::Subscription::none();
    }

    cosmic::iced::Subscription::run_with((TypeId::of::<IdleSubscriptionMarker>(), config), |data| {
        let config = data.1.clone();
        cosmic::iced::stream::channel(
            8,
            |mut output: cosmic::iced_futures::futures::channel::mpsc::Sender<IdleEvent>| async move {
                use cosmic::iced_futures::futures::SinkExt;

                // Run the blocking Wayland event loop in a dedicated thread
                let (tx, rx) = std_mpsc::channel();

                let config_clone = config;
                let handle = tokio::task::spawn_blocking(move || {
                    run_wayland_idle_loop(config_clone, tx);
                });

                // Bridge std_mpsc → iced channel
                loop {
                    // Poll for events with a short sleep to avoid busy-waiting
                    match rx.try_recv() {
                        Ok(event) => {
                            if output.send(event).await.is_err() {
                                break;
                            }
                        }
                        Err(std_mpsc::TryRecvError::Disconnected) => {
                            // Wayland thread exited — send error
                            let _ = output
                                .send(IdleEvent::Error("Wayland idle thread exited".to_string()))
                                .await;
                            break;
                        }
                        Err(std_mpsc::TryRecvError::Empty) => {
                            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
                        }
                    }
                }

                // Ensure the blocking task is cleaned up
                handle.abort();

                // Keep the future alive so iced doesn't immediately restart
                std::future::pending::<()>().await;
            },
        )
    })
}

/// Marker type for subscription deduplication
struct IdleSubscriptionMarker;

/// Blocking Wayland event loop that runs in `spawn_blocking`.
#[allow(clippy::needless_pass_by_value)] // Owned for spawn_blocking move
fn run_wayland_idle_loop(config: IdleSubscriptionConfig, tx: std_mpsc::Sender<IdleEvent>) {
    let conn = match Connection::connect_to_env() {
        Ok(c) => c,
        Err(e) => {
            let _ = tx.send(IdleEvent::Error(format!("Wayland connection failed: {e}")));
            return;
        }
    };

    let display = conn.display();
    let mut event_queue = conn.new_event_queue::<IdleState>();
    let qh = event_queue.handle();

    let mut state = IdleState {
        tx: tx.clone(),
        notifier: None,
        seat: None,
    };

    // Get the registry and do a roundtrip to discover globals
    let _registry = display.get_registry(&qh, ());
    if event_queue.roundtrip(&mut state).is_err() {
        let _ = tx.send(IdleEvent::Error(
            "Wayland registry roundtrip failed".to_string(),
        ));
        return;
    }

    let (notifier, seat) = match (state.notifier.as_ref(), state.seat.as_ref()) {
        (Some(n), Some(s)) => (n, s),
        (None, _) => {
            let _ = tx.send(IdleEvent::Error(
                "ext_idle_notifier_v1 not available".to_string(),
            ));
            return;
        }
        (_, None) => {
            let _ = tx.send(IdleEvent::Error("No wl_seat found".to_string()));
            return;
        }
    };

    // Create idle notification objects
    if config.screensaver_timeout_ms > 0 {
        let _screensaver_notif = notifier.get_idle_notification(
            config.screensaver_timeout_ms,
            seat,
            &qh,
            NotificationKind::Screensaver,
        );
    }

    if config.lock_timeout_ms > 0 {
        let _lock_notif = notifier.get_idle_notification(
            config.lock_timeout_ms,
            seat,
            &qh,
            NotificationKind::Lock,
        );
    }

    // Flush the requests
    if conn.flush().is_err() {
        let _ = tx.send(IdleEvent::Error("Wayland flush failed".to_string()));
        return;
    }

    // Signal successful connection
    let _ = tx.send(IdleEvent::Connected);

    // Pump events until the channel is closed or an error occurs
    loop {
        match event_queue.blocking_dispatch(&mut state) {
            Ok(_) => {}
            Err(e) => {
                let _ = tx.send(IdleEvent::Error(format!("Wayland dispatch error: {e}")));
                break;
            }
        }
    }
}
