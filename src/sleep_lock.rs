// SPDX-License-Identifier: GPL-3.0-only

//! Sleep lock subscription via logind D-Bus
//!
//! Subscribes to `org.freedesktop.login1.Manager.PrepareForSleep` so we can
//! lock the screen before the system suspends.

use std::any::TypeId;
use std::time::Duration;

/// Events from the sleep lock subscription
#[derive(Debug, Clone)]
pub enum SleepEvent {
    /// System is about to sleep — lock the screen
    PrepareForSleep,
}

/// Create a libcosmic subscription that emits `PrepareForSleep` before suspend.
pub fn sleep_lock_subscription() -> cosmic::iced::Subscription<SleepEvent> {
    cosmic::iced::Subscription::run_with(TypeId::of::<SleepLockMarker>(), |_| {
        cosmic::iced::stream::channel(4, |mut output| async move {
            loop {
                match run_sleep_monitor(&mut output).await {
                    Ok(()) => break, // channel closed
                    Err(e) => {
                        tracing::warn!("Sleep lock subscription error, retrying: {e}");
                        tokio::time::sleep(Duration::from_secs(5)).await;
                    }
                }
            }
        })
    })
}

/// Marker type for subscription deduplication
struct SleepLockMarker;

/// Monitor logind `PrepareForSleep` signal via `MessageStream`.
async fn run_sleep_monitor(
    output: &mut cosmic::iced_futures::futures::channel::mpsc::Sender<SleepEvent>,
) -> Result<(), String> {
    use cosmic::iced_futures::futures::{SinkExt, StreamExt};

    let connection = zbus::Connection::system()
        .await
        .map_err(|e| format!("System D-Bus connection failed: {e}"))?;

    let rule = zbus::MatchRule::builder()
        .msg_type(zbus::message::Type::Signal)
        .sender("org.freedesktop.login1")
        .map_err(|e| format!("Invalid sender: {e}"))?
        .interface("org.freedesktop.login1.Manager")
        .map_err(|e| format!("Invalid interface: {e}"))?
        .member("PrepareForSleep")
        .map_err(|e| format!("Invalid member: {e}"))?
        .build();

    let mut stream = zbus::MessageStream::for_match_rule(rule, &connection, Some(4))
        .await
        .map_err(|e| format!("MessageStream setup failed: {e}"))?;

    tracing::info!("Sleep lock subscription connected");

    loop {
        match stream.next().await {
            Some(Ok(msg)) => {
                // PrepareForSleep(bool) — true means going to sleep
                if let Ok((going_to_sleep,)) = msg.body().deserialize::<(bool,)>()
                    && going_to_sleep
                {
                    tracing::info!("PrepareForSleep(true) — locking screen");
                    if output.send(SleepEvent::PrepareForSleep).await.is_err() {
                        return Ok(()); // channel closed
                    }
                }
            }
            Some(Err(e)) => {
                return Err(format!("Signal stream error: {e}"));
            }
            None => {
                return Err("Signal stream ended".to_string());
            }
        }
    }
}
