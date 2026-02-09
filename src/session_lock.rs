// SPDX-License-Identifier: GPL-3.0-only

//! Native session lock via ext-session-lock-v1 Wayland protocol
//!
//! Acquires a session lock, renders solid-color lock surfaces on all outputs,
//! and unlocks on any keypress. Falls back to logind D-Bus when the protocol
//! is unavailable.

use std::any::TypeId;
use std::collections::HashMap;
use std::io::{BufWriter, Write};
use std::os::unix::io::AsFd;
use std::sync::mpsc as std_mpsc;

use wayland_client::protocol::{
    wl_buffer, wl_compositor, wl_keyboard, wl_output, wl_registry, wl_seat, wl_shm, wl_shm_pool,
    wl_surface,
};
use wayland_client::{Connection, Dispatch, QueueHandle, delegate_noop};
use wayland_protocols::ext::session_lock::v1::client::{
    ext_session_lock_manager_v1::ExtSessionLockManagerV1,
    ext_session_lock_surface_v1::{self, ExtSessionLockSurfaceV1},
    ext_session_lock_v1::{self, ExtSessionLockV1},
};

use crate::colors::hex_to_rgb;

/// Events emitted by the session lock subscription
#[derive(Debug, Clone)]
pub enum SessionLockEvent {
    /// Lock successfully acquired (compositor acknowledged)
    Locked,
    /// User pressed a key — unlock requested
    UnlockRequested,
    /// Lock acquisition failed (compositor rejected or protocol error)
    Failed(String),
    /// Protocol unavailable or connection error
    Error(String),
}

/// Per-output lock surface data
struct LockSurfaceData {
    surface: wl_surface::WlSurface,
    lock_surface: ExtSessionLockSurfaceV1,
}

/// Wayland dispatch state for session lock
struct LockState {
    tx: std_mpsc::Sender<SessionLockEvent>,
    compositor: Option<wl_compositor::WlCompositor>,
    shm: Option<wl_shm::WlShm>,
    seat: Option<wl_seat::WlSeat>,
    lock_manager: Option<ExtSessionLockManagerV1>,
    lock: Option<ExtSessionLockV1>,
    outputs: HashMap<u32, wl_output::WlOutput>,
    lock_surfaces: HashMap<u32, LockSurfaceData>,
    locked: bool,
    unlock_requested: bool,
    bg_r: u8,
    bg_g: u8,
    bg_b: u8,
}

// --- Dispatch implementations ---

impl Dispatch<wl_registry::WlRegistry, ()> for LockState {
    fn event(
        state: &mut Self,
        registry: &wl_registry::WlRegistry,
        event: wl_registry::Event,
        _data: &(),
        _conn: &Connection,
        qh: &QueueHandle<Self>,
    ) {
        match event {
            wl_registry::Event::Global {
                name,
                interface,
                version,
            } => match interface.as_str() {
                "wl_compositor" => {
                    state.compositor = Some(registry.bind::<wl_compositor::WlCompositor, _, Self>(
                        name,
                        version,
                        qh,
                        (),
                    ));
                }
                "wl_shm" => {
                    state.shm =
                        Some(registry.bind::<wl_shm::WlShm, _, Self>(name, version, qh, ()));
                }
                "wl_seat" => {
                    if state.seat.is_none() {
                        state.seat =
                            Some(registry.bind::<wl_seat::WlSeat, _, Self>(name, version, qh, ()));
                    }
                }
                "wl_output" => {
                    let output =
                        registry.bind::<wl_output::WlOutput, _, Self>(name, version, qh, name);
                    state.outputs.insert(name, output);
                }
                "ext_session_lock_manager_v1" => {
                    state.lock_manager = Some(registry.bind::<ExtSessionLockManagerV1, _, Self>(
                        name,
                        version,
                        qh,
                        (),
                    ));
                }
                _ => {}
            },
            wl_registry::Event::GlobalRemove { name } => {
                if let Some(data) = state.lock_surfaces.remove(&name) {
                    data.lock_surface.destroy();
                    data.surface.destroy();
                }
                state.outputs.remove(&name);
            }
            _ => {}
        }
    }
}

impl Dispatch<ext_session_lock_v1::ExtSessionLockV1, ()> for LockState {
    fn event(
        state: &mut Self,
        _proxy: &ExtSessionLockV1,
        event: ext_session_lock_v1::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        match event {
            ext_session_lock_v1::Event::Locked => {
                state.locked = true;
                let _ = state.tx.send(SessionLockEvent::Locked);
            }
            ext_session_lock_v1::Event::Finished => {
                let _ = state.tx.send(SessionLockEvent::Failed(
                    "Lock finished by compositor".to_string(),
                ));
            }
            _ => {}
        }
    }
}

impl Dispatch<ExtSessionLockSurfaceV1, u32> for LockState {
    fn event(
        state: &mut Self,
        proxy: &ExtSessionLockSurfaceV1,
        event: ext_session_lock_surface_v1::Event,
        _output_name: &u32,
        _conn: &Connection,
        qh: &QueueHandle<Self>,
    ) {
        if let ext_session_lock_surface_v1::Event::Configure {
            serial,
            width,
            height,
        } = event
        {
            proxy.ack_configure(serial);

            // Find the wl_surface for this lock surface and render
            for data in state.lock_surfaces.values() {
                if data.lock_surface == *proxy {
                    if let Some(ref shm) = state.shm {
                        render_solid_color(
                            &data.surface,
                            shm,
                            width,
                            height,
                            state.bg_r,
                            state.bg_g,
                            state.bg_b,
                            qh,
                        );
                    }
                    break;
                }
            }
        }
    }
}

impl Dispatch<wl_seat::WlSeat, ()> for LockState {
    fn event(
        _state: &mut Self,
        seat: &wl_seat::WlSeat,
        event: wl_seat::Event,
        _data: &(),
        _conn: &Connection,
        qh: &QueueHandle<Self>,
    ) {
        if let wl_seat::Event::Capabilities {
            capabilities: cap_raw,
        } = event
        {
            let cap = wl_seat::Capability::from_bits_truncate(cap_raw.into());
            if cap.contains(wl_seat::Capability::Keyboard) {
                seat.get_keyboard(qh, ());
            }
        }
    }
}

impl Dispatch<wl_keyboard::WlKeyboard, ()> for LockState {
    fn event(
        state: &mut Self,
        _proxy: &wl_keyboard::WlKeyboard,
        event: wl_keyboard::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        if let wl_keyboard::Event::Key {
            state: key_state, ..
        } = event
        {
            if key_state == wayland_client::WEnum::Value(wl_keyboard::KeyState::Pressed)
                && state.locked
            {
                state.unlock_requested = true;
                let _ = state.tx.send(SessionLockEvent::UnlockRequested);
            }
        }
    }
}

impl Dispatch<wl_output::WlOutput, u32> for LockState {
    fn event(
        _state: &mut Self,
        _proxy: &wl_output::WlOutput,
        _event: wl_output::Event,
        _name: &u32,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
    }
}

delegate_noop!(LockState: ignore wl_compositor::WlCompositor);
delegate_noop!(LockState: ignore wl_shm::WlShm);
delegate_noop!(LockState: ignore wl_shm_pool::WlShmPool);
delegate_noop!(LockState: ignore wl_buffer::WlBuffer);
delegate_noop!(LockState: ignore wl_surface::WlSurface);
delegate_noop!(LockState: ignore ExtSessionLockManagerV1);

/// Render a solid color ARGB8888 buffer and attach it to the surface.
fn render_solid_color(
    surface: &wl_surface::WlSurface,
    shm: &wl_shm::WlShm,
    width: u32,
    height: u32,
    r: u8,
    g: u8,
    b: u8,
    qh: &QueueHandle<LockState>,
) {
    let stride = width as i32 * 4;
    let size = stride * height as i32;

    let file = match tempfile::tempfile() {
        Ok(f) => f,
        Err(e) => {
            tracing::error!("Failed to create tempfile for SHM buffer: {e}");
            return;
        }
    };

    if file.set_len(size as u64).is_err() {
        tracing::error!("Failed to set tempfile size");
        return;
    }

    let mut writer = BufWriter::new(&file);
    let pixel = [b, g, r, 0xFF]; // BGRA byte order for ARGB8888 on little-endian
    for _ in 0..(width * height) {
        if writer.write_all(&pixel).is_err() {
            tracing::error!("Failed to write pixel data");
            return;
        }
    }
    if writer.flush().is_err() {
        tracing::error!("Failed to flush pixel data");
        return;
    }

    let pool = shm.create_pool(file.as_fd(), size, qh, ());
    let buffer = pool.create_buffer(
        0,
        width as i32,
        height as i32,
        stride,
        wl_shm::Format::Argb8888,
        qh,
        (),
    );

    surface.attach(Some(&buffer), 0, 0);
    surface.damage_buffer(0, 0, width as i32, height as i32);
    surface.commit();
}

/// Create a libcosmic subscription that acquires a session lock.
///
/// The subscription is one-shot: it locks, waits for unlock, then terminates.
pub fn lock_session(bg_color: String) -> cosmic::iced::Subscription<SessionLockEvent> {
    cosmic::iced::Subscription::run_with_id(
        TypeId::of::<SessionLockMarker>(),
        cosmic::iced::stream::channel(8, |mut output| async move {
            use cosmic::iced_futures::futures::SinkExt;

            let (tx, rx) = std_mpsc::channel();

            let handle = tokio::task::spawn_blocking(move || {
                run_session_lock_loop(bg_color, tx);
            });

            // Bridge std_mpsc → iced channel
            loop {
                match rx.try_recv() {
                    Ok(event) => {
                        let is_terminal = matches!(
                            event,
                            SessionLockEvent::UnlockRequested
                                | SessionLockEvent::Failed(_)
                                | SessionLockEvent::Error(_)
                        );
                        if output.send(event).await.is_err() {
                            break;
                        }
                        if is_terminal {
                            break;
                        }
                    }
                    Err(std_mpsc::TryRecvError::Disconnected) => {
                        let _ = output
                            .send(SessionLockEvent::Error(
                                "Session lock thread exited".to_string(),
                            ))
                            .await;
                        break;
                    }
                    Err(std_mpsc::TryRecvError::Empty) => {
                        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
                    }
                }
            }

            handle.abort();

            // Keep future alive so iced doesn't immediately restart
            std::future::pending::<()>().await;
        }),
    )
}

/// Marker type for subscription deduplication
struct SessionLockMarker;

/// Blocking Wayland event loop for session lock
fn run_session_lock_loop(bg_color: String, tx: std_mpsc::Sender<SessionLockEvent>) {
    let (bg_r, bg_g, bg_b) = hex_to_rgb(&bg_color).unwrap_or((27, 27, 27));

    let conn = match Connection::connect_to_env() {
        Ok(c) => c,
        Err(e) => {
            let _ = tx.send(SessionLockEvent::Error(format!(
                "Wayland connection failed: {e}"
            )));
            return;
        }
    };

    let display = conn.display();
    let mut event_queue = conn.new_event_queue::<LockState>();
    let qh = event_queue.handle();

    let mut state = LockState {
        tx: tx.clone(),
        compositor: None,
        shm: None,
        seat: None,
        lock_manager: None,
        lock: None,
        outputs: HashMap::new(),
        lock_surfaces: HashMap::new(),
        locked: false,
        unlock_requested: false,
        bg_r,
        bg_g,
        bg_b,
    };

    // Discover globals
    let _registry = display.get_registry(&qh, ());
    if event_queue.roundtrip(&mut state).is_err() {
        let _ = tx.send(SessionLockEvent::Error(
            "Wayland registry roundtrip failed".to_string(),
        ));
        return;
    }

    // Second roundtrip to bind outputs
    if event_queue.roundtrip(&mut state).is_err() {
        let _ = tx.send(SessionLockEvent::Error(
            "Wayland output roundtrip failed".to_string(),
        ));
        return;
    }

    // Validate required globals
    let lock_manager = match state.lock_manager.as_ref() {
        Some(m) => m.clone(),
        None => {
            let _ = tx.send(SessionLockEvent::Error(
                "ext_session_lock_manager_v1 not available".to_string(),
            ));
            return;
        }
    };

    let compositor = match state.compositor.as_ref() {
        Some(c) => c.clone(),
        None => {
            let _ = tx.send(SessionLockEvent::Error(
                "wl_compositor not available".to_string(),
            ));
            return;
        }
    };

    // Acquire the lock
    let lock = lock_manager.lock(&qh, ());
    state.lock = Some(lock.clone());

    // Create lock surfaces for all known outputs
    let output_names: Vec<u32> = state.outputs.keys().copied().collect();
    for name in output_names {
        if let Some(output) = state.outputs.get(&name) {
            let surface = compositor.create_surface(&qh, ());
            let lock_surface = lock.get_lock_surface(&surface, output, &qh, name);
            state.lock_surfaces.insert(
                name,
                LockSurfaceData {
                    surface,
                    lock_surface,
                },
            );
        }
    }

    if conn.flush().is_err() {
        let _ = tx.send(SessionLockEvent::Error("Wayland flush failed".to_string()));
        return;
    }

    // Event loop — pump until unlock requested
    loop {
        match event_queue.blocking_dispatch(&mut state) {
            Ok(_) => {
                if state.unlock_requested {
                    do_unlock(&mut state);
                    break;
                }
            }
            Err(e) => {
                let _ = tx.send(SessionLockEvent::Error(format!(
                    "Wayland dispatch error: {e}"
                )));
                break;
            }
        }
    }
}

/// Clean up lock surfaces and unlock
fn do_unlock(state: &mut LockState) {
    for (_, data) in state.lock_surfaces.drain() {
        data.lock_surface.destroy();
        data.surface.destroy();
    }

    if let Some(lock) = state.lock.take() {
        lock.unlock_and_destroy();
    }
}
