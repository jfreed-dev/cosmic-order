// SPDX-License-Identifier: GPL-3.0-only

//! Power state monitoring via D-Bus (UPower + system76-power)
//!
//! Provides live power state tracking through D-Bus subscriptions so the
//! screensaver can adapt effect complexity to the current power situation.

use std::fmt;
use std::time::Duration;

use crate::fl;

/// Current system power state
#[derive(Debug, Clone, Default, PartialEq)]
pub struct PowerState {
    /// Whether the system is running on battery
    pub on_battery: bool,
    /// Battery charge percentage (None if no battery present)
    pub battery_percent: Option<u8>,
    /// Current power profile from system76-power
    pub power_profile: PowerProfile,
    /// Whether system76-power daemon is available
    pub has_system76_power: bool,
}

impl PowerState {
    /// Determine the appropriate effect profile based on power state
    pub fn effect_profile(&self) -> EffectProfile {
        match (self.on_battery, self.battery_percent) {
            // Critical battery — skip screensaver entirely
            (true, Some(pct)) if pct < 10 => EffectProfile::Skip,
            // Very low battery — minimal effects only
            (true, Some(pct)) if pct < 20 => EffectProfile::Minimal,
            // Battery below 50% — simple effects
            (true, Some(pct)) if pct < 50 => EffectProfile::Simple,
            // On battery with decent charge — standard
            (true, _) => EffectProfile::Standard,
            // AC + Performance profile — full effects
            (false, _) if self.power_profile == PowerProfile::Performance => EffectProfile::Full,
            // AC otherwise — standard
            (false, _) => EffectProfile::Standard,
        }
    }

    /// Format power state for UI display using localized strings
    #[allow(dead_code)]
    pub fn display_string(&self) -> String {
        let source = match (self.on_battery, self.battery_percent) {
            (false, None) => fl!("screensaver-power-no-battery"),
            (false, Some(_)) => fl!("screensaver-power-ac"),
            (true, Some(pct)) => fl!("screensaver-power-battery", percent = pct.to_string()),
            (true, None) => fl!("screensaver-power-battery", percent = "?".to_string()),
        };

        let profile = match self.power_profile {
            PowerProfile::Performance => fl!("screensaver-power-profile-performance"),
            PowerProfile::Balanced => fl!("screensaver-power-profile-balanced"),
            PowerProfile::PowerSaver => fl!("screensaver-power-profile-powersaver"),
        };

        format!("{source} · {profile}")
    }

    /// Format power state as shell KEY="value" pairs for screensaver-ctl
    pub fn to_env_format(&self) -> String {
        let on_battery = if self.on_battery { "true" } else { "false" };
        let percent = self
            .battery_percent
            .map_or_else(String::new, |p| p.to_string());
        let profile = match self.power_profile {
            PowerProfile::Performance => "performance",
            PowerProfile::Balanced => "balanced",
            PowerProfile::PowerSaver => "power-saver",
        };
        let effect = self.effect_profile();

        format!(
            "ON_BATTERY=\"{on_battery}\"\n\
             BATTERY_PERCENT=\"{percent}\"\n\
             POWER_PROFILE=\"{profile}\"\n\
             EFFECT_PROFILE=\"{effect}\"\n"
        )
    }
}

/// System power profile (from system76-power or fallback)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PowerProfile {
    Performance,
    #[default]
    Balanced,
    PowerSaver,
}

/// Effect complexity profile determined by power state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EffectProfile {
    /// All effects enabled, no restrictions
    Full,
    /// Normal effect set
    Standard,
    /// Reduced complexity effects
    Simple,
    /// Only lightweight effects
    Minimal,
    /// Skip screensaver entirely (critical battery)
    Skip,
}

impl fmt::Display for EffectProfile {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Full => write!(f, "full"),
            Self::Standard => write!(f, "standard"),
            Self::Simple => write!(f, "simple"),
            Self::Minimal => write!(f, "minimal"),
            Self::Skip => write!(f, "skip"),
        }
    }
}

impl EffectProfile {
    /// Localized display name for the UI
    #[allow(dead_code)]
    pub fn display_name(self) -> String {
        match self {
            Self::Full => fl!("screensaver-effect-full"),
            Self::Standard => fl!("screensaver-effect-standard"),
            Self::Simple => fl!("screensaver-effect-simple"),
            Self::Minimal => fl!("screensaver-effect-minimal"),
            Self::Skip => fl!("screensaver-effect-skip"),
        }
    }
}

// ---------------------------------------------------------------------------
// D-Bus queries
// ---------------------------------------------------------------------------

/// One-shot query of current power state via D-Bus
pub async fn query_power_state() -> PowerState {
    let mut state = PowerState::default();

    // Query UPower
    match query_upower().await {
        Ok((on_battery, percent)) => {
            state.on_battery = on_battery;
            state.battery_percent = percent;
        }
        Err(e) => {
            tracing::warn!("UPower query failed, using defaults: {e}");
        }
    }

    // Query system76-power profile
    match query_system76_power().await {
        Ok(profile) => {
            state.power_profile = profile;
            state.has_system76_power = true;
        }
        Err(e) => {
            tracing::debug!("system76-power not available: {e}");
        }
    }

    state
}

/// Helper to read a D-Bus property via the Properties interface
async fn get_dbus_property(
    connection: &zbus::Connection,
    dest: &str,
    path: &str,
    iface: &str,
    prop: &str,
) -> Result<zbus::zvariant::OwnedValue, zbus::Error> {
    let reply = connection
        .call_method(
            Some(dest),
            path,
            Some("org.freedesktop.DBus.Properties"),
            "Get",
            &(iface, prop),
        )
        .await?;
    reply.body().deserialize()
}

/// Query UPower DisplayDevice for battery state
async fn query_upower() -> Result<(bool, Option<u8>), zbus::Error> {
    let connection = zbus::Connection::system().await?;

    // Read OnBattery from the UPower daemon itself
    let on_battery_val = get_dbus_property(
        &connection,
        "org.freedesktop.UPower",
        "/org/freedesktop/UPower",
        "org.freedesktop.UPower",
        "OnBattery",
    )
    .await?;
    let on_battery = on_battery_val.downcast_ref::<bool>().unwrap_or(false);

    // Check device Type to determine if a battery exists
    // Type 2 = Battery, 0 = Unknown (no battery)
    let device_type = get_dbus_property(
        &connection,
        "org.freedesktop.UPower",
        "/org/freedesktop/UPower/devices/DisplayDevice",
        "org.freedesktop.UPower.Device",
        "Type",
    )
    .await
    .ok()
    .and_then(|v| v.downcast_ref::<u32>().ok())
    .unwrap_or(0);

    if device_type == 0 {
        // No battery present
        return Ok((on_battery, None));
    }

    // Read battery percentage from DisplayDevice
    let percentage = get_dbus_property(
        &connection,
        "org.freedesktop.UPower",
        "/org/freedesktop/UPower/devices/DisplayDevice",
        "org.freedesktop.UPower.Device",
        "Percentage",
    )
    .await
    .ok()
    .and_then(|v| v.downcast_ref::<f64>().ok())
    .map(|p| p.clamp(0.0, 100.0) as u8);

    Ok((on_battery, percentage))
}

/// Query system76-power for current power profile
async fn query_system76_power() -> Result<PowerProfile, zbus::Error> {
    let connection = zbus::Connection::system().await?;

    let reply = connection
        .call_method(
            Some("com.system76.PowerDaemon"),
            "/com/system76/PowerDaemon",
            Some("com.system76.PowerDaemon"),
            "GetProfile",
            &(),
        )
        .await?;

    let profile_str: String = reply.body().deserialize()?;

    Ok(match profile_str.as_str() {
        "Performance" => PowerProfile::Performance,
        "Battery" => PowerProfile::PowerSaver,
        _ => PowerProfile::Balanced,
    })
}

// ---------------------------------------------------------------------------
// Subscription (long-running D-Bus signal monitoring)
// ---------------------------------------------------------------------------

/// Create a libcosmic subscription that yields `PowerState` on changes
pub fn power_subscription() -> cosmic::iced::Subscription<PowerState> {
    cosmic::iced::Subscription::run_with_id(
        std::any::TypeId::of::<PowerSubscriptionMarker>(),
        cosmic::iced::stream::channel(4, |mut output| async move {
            use cosmic::iced_futures::futures::SinkExt;

            // Initial query
            let mut current = query_power_state().await;
            if output.send(current.clone()).await.is_err() {
                return;
            }

            // Subscribe to D-Bus property changes
            loop {
                match run_signal_loop(&mut output, &mut current).await {
                    Ok(()) => break, // channel closed
                    Err(e) => {
                        tracing::warn!("Power subscription error, reconnecting: {e}");
                        tokio::time::sleep(Duration::from_secs(5)).await;
                        // Re-query and send updated state
                        let fresh = query_power_state().await;
                        if fresh != current {
                            current = fresh;
                            if output.send(current.clone()).await.is_err() {
                                return;
                            }
                        }
                    }
                }
            }
        }),
    )
}

/// Marker type for subscription deduplication
struct PowerSubscriptionMarker;

/// Inner polling loop — re-queries D-Bus periodically and emits on change.
///
/// A polling approach is used because D-Bus signal subscription requires
/// complex match rule setup that varies across zbus versions. Polling every
/// 5 seconds is lightweight and ensures reliable detection of power changes.
async fn run_signal_loop(
    output: &mut cosmic::iced_futures::futures::channel::mpsc::Sender<PowerState>,
    current: &mut PowerState,
) -> Result<(), zbus::Error> {
    use cosmic::iced_futures::futures::SinkExt;

    loop {
        tokio::time::sleep(Duration::from_secs(5)).await;

        let fresh = query_power_state().await;
        if fresh != *current {
            *current = fresh.clone();
            if output.send(fresh).await.is_err() {
                return Ok(()); // channel closed
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn test_effect_profile_logic() {
        // Critical battery → Skip
        let state = PowerState {
            on_battery: true,
            battery_percent: Some(5),
            power_profile: PowerProfile::Balanced,
            has_system76_power: false,
        };
        assert_eq!(state.effect_profile(), EffectProfile::Skip);

        // Very low battery → Minimal
        let state = PowerState {
            on_battery: true,
            battery_percent: Some(15),
            power_profile: PowerProfile::Balanced,
            has_system76_power: false,
        };
        assert_eq!(state.effect_profile(), EffectProfile::Minimal);

        // Battery below 50% → Simple
        let state = PowerState {
            on_battery: true,
            battery_percent: Some(45),
            power_profile: PowerProfile::Balanced,
            has_system76_power: false,
        };
        assert_eq!(state.effect_profile(), EffectProfile::Simple);

        // Battery above 50% → Standard
        let state = PowerState {
            on_battery: true,
            battery_percent: Some(72),
            power_profile: PowerProfile::Balanced,
            has_system76_power: false,
        };
        assert_eq!(state.effect_profile(), EffectProfile::Standard);

        // AC + Performance → Full
        let state = PowerState {
            on_battery: false,
            battery_percent: Some(100),
            power_profile: PowerProfile::Performance,
            has_system76_power: true,
        };
        assert_eq!(state.effect_profile(), EffectProfile::Full);

        // AC + Balanced → Standard
        let state = PowerState {
            on_battery: false,
            battery_percent: Some(80),
            power_profile: PowerProfile::Balanced,
            has_system76_power: true,
        };
        assert_eq!(state.effect_profile(), EffectProfile::Standard);

        // AC + no battery → Standard
        let state = PowerState {
            on_battery: false,
            battery_percent: None,
            power_profile: PowerProfile::Balanced,
            has_system76_power: false,
        };
        assert_eq!(state.effect_profile(), EffectProfile::Standard);

        // AC + PowerSaver → Standard (not Full — only Performance gets Full)
        let state = PowerState {
            on_battery: false,
            battery_percent: Some(90),
            power_profile: PowerProfile::PowerSaver,
            has_system76_power: true,
        };
        assert_eq!(state.effect_profile(), EffectProfile::Standard);

        // Boundary: battery exactly 10% → Minimal (not Skip)
        let state = PowerState {
            on_battery: true,
            battery_percent: Some(10),
            power_profile: PowerProfile::Balanced,
            has_system76_power: false,
        };
        assert_eq!(state.effect_profile(), EffectProfile::Minimal);

        // Boundary: battery exactly 20% → Simple (not Minimal)
        let state = PowerState {
            on_battery: true,
            battery_percent: Some(20),
            power_profile: PowerProfile::Balanced,
            has_system76_power: false,
        };
        assert_eq!(state.effect_profile(), EffectProfile::Simple);

        // Boundary: battery exactly 50% → Standard (not Simple)
        let state = PowerState {
            on_battery: true,
            battery_percent: Some(50),
            power_profile: PowerProfile::Balanced,
            has_system76_power: false,
        };
        assert_eq!(state.effect_profile(), EffectProfile::Standard);
    }

    #[test]
    fn test_env_format() {
        let state = PowerState {
            on_battery: true,
            battery_percent: Some(45),
            power_profile: PowerProfile::PowerSaver,
            has_system76_power: true,
        };
        let env = state.to_env_format();

        assert!(env.contains("ON_BATTERY=\"true\""));
        assert!(env.contains("BATTERY_PERCENT=\"45\""));
        assert!(env.contains("POWER_PROFILE=\"power-saver\""));
        assert!(env.contains("EFFECT_PROFILE=\"simple\""));

        // AC with no battery
        let state = PowerState {
            on_battery: false,
            battery_percent: None,
            power_profile: PowerProfile::Balanced,
            has_system76_power: false,
        };
        let env = state.to_env_format();

        assert!(env.contains("ON_BATTERY=\"false\""));
        assert!(env.contains("BATTERY_PERCENT=\"\""));
        assert!(env.contains("POWER_PROFILE=\"balanced\""));
        assert!(env.contains("EFFECT_PROFILE=\"standard\""));
    }

    #[test]
    fn test_effect_profile_display() {
        assert_eq!(EffectProfile::Full.to_string(), "full");
        assert_eq!(EffectProfile::Standard.to_string(), "standard");
        assert_eq!(EffectProfile::Simple.to_string(), "simple");
        assert_eq!(EffectProfile::Minimal.to_string(), "minimal");
        assert_eq!(EffectProfile::Skip.to_string(), "skip");
    }

    #[test]
    fn test_default_power_state() {
        let state = PowerState::default();
        assert!(!state.on_battery);
        assert_eq!(state.battery_percent, None);
        assert_eq!(state.power_profile, PowerProfile::Balanced);
        assert!(!state.has_system76_power);
        assert_eq!(state.effect_profile(), EffectProfile::Standard);
    }
}
