//! Wayland auto-paste via the XDG RemoteDesktop portal + libei.
//!
//! Extracted and trimmed from the sibling `wdotool` project
//! (`wdotool-core`'s libei backend). Sends a synthetic Ctrl+V *through the
//! compositor* — no `/dev/uinput`, no `ydotool` daemon, no root. This is the
//! replacement for the old ydotool paste path, which needed `ydotoold`
//! holding `/dev/uinput` open and bypassed the compositor entirely.
//!
//! Flow: open an `org.freedesktop.portal.RemoteDesktop` session, select only
//! the Keyboard device, handshake libei as a *sender*, then emit key events
//! through the compositor's own libei implementation. The portal asks for
//! consent once ("Allow this app to control your computer?"); the issued
//! `restore_token` is cached (mode 0600) so every later paste is silent until
//! the user revokes it from the desktop's privacy settings.
//!
//! Keyboard-only by design: we never request pointer control, so the consent
//! grant is limited to keystrokes.
//!
//! Threading: the libei event stream isn't `Send` (reis stores non-`Send`
//! callbacks), so all async work — the portal negotiation *and* the event
//! loop — runs on one dedicated OS thread with its own current-thread tokio
//! runtime. The public API is fully synchronous and depends on no ambient
//! runtime; the caller (the paste worker thread) just blocks on std channels.
//! Emit calls run on the caller's thread through the `Connection` proxy, which
//! is `Send + Sync`.

use std::io::Write as _;
use std::os::unix::fs::{DirBuilderExt, OpenOptionsExt};
use std::os::unix::net::UnixStream;
use std::path::PathBuf;
use std::sync::{mpsc, Arc, Mutex, OnceLock};
use std::time::{Duration, Instant};

use ashpd::desktop::remote_desktop::{
    ConnectToEISOptions, DeviceType, RemoteDesktop, SelectDevicesOptions, SelectedDevices,
    StartOptions,
};
use ashpd::desktop::{CreateSessionOptions, PersistMode, Session};
use enumflags2::BitFlags;
use futures_util::StreamExt;
use reis::ei;
use reis::event::{self as rev, DeviceCapability, EiEvent};
use serde::{Deserialize, Serialize};
use xkbcommon::xkb;

/// Reverse-domain identifier shown to the portal and sent to the compositor
/// as the provenance of the synthetic input. Matches the app's Tauri id.
const APP_ID: &str = "com.datluong.linuxclipboard";

/// Time budget for the whole portal negotiation, *including* the one-time
/// consent dialog on first run — so it's generous. The paste worker is a
/// detached thread and the clipboard is already set by the time it runs, so a
/// long wait here never blocks the UI (auto-paste just doesn't fire).
const CONNECT_TIMEOUT_SECS: u64 = 60;

/// Time budget for a keyboard device to resume *after* the portal accepted.
const READY_TIMEOUT_SECS: u64 = 15;

/// Lazily-built, shared portal session. Stored in `AppState` and cloned into
/// the paste thread; the first paste builds the session (and triggers the
/// one-time consent dialog), later pastes reuse the live connection.
pub type PortalCell = Arc<Mutex<Option<Arc<PortalPaster>>>>;

pub fn new_cell() -> PortalCell {
    Arc::new(Mutex::new(None))
}

/// Send Ctrl+V through a (lazily-built) portal session. Returns `true` if the
/// keystroke was dispatched. Never panics or errors hard: any failure returns
/// `false` so the caller can fall back to copy-only.
pub fn paste_ctrl_v(cell: &PortalCell) -> bool {
    let Some(paster) = get_or_build(cell) else {
        return false;
    };
    match paster.send_ctrl_v() {
        Ok(()) => true,
        Err(e) => {
            eprintln!("[portal] paste failed: {e}");
            false
        }
    }
}

/// True if the portal paste permission looks granted: either a live session is
/// held this run, or a restore token was cached from a previous grant. (If the
/// user revoked consent in the system settings the cached token is stale; the
/// next enable/paste re-prompts and reconciles.)
pub fn is_granted(cell: &PortalCell) -> bool {
    if cell.lock().unwrap().is_some() {
        return true;
    }
    token_path().map(|p| p.exists()).unwrap_or(false)
}

/// Trigger the consent flow (or silently reuse a cached grant) and build the
/// session. Returns true if a usable paste session is now available. Blocks up
/// to ~`CONNECT_TIMEOUT_SECS` while the consent dialog is up.
pub fn enable(cell: &PortalCell) -> bool {
    get_or_build(cell).is_some()
}

/// Return the cached paster, building (and caching) it on first use.
fn get_or_build(cell: &PortalCell) -> Option<Arc<PortalPaster>> {
    // Fast path: already built.
    if let Some(p) = cell.lock().unwrap().clone() {
        return Some(p);
    }
    match PortalPaster::build_blocking() {
        Ok(p) => {
            let arc = Arc::new(p);
            let mut guard = cell.lock().unwrap();
            // If a concurrent first-paste won the race, keep the winner and let
            // ours drop.
            Some(guard.get_or_insert(arc).clone())
        }
        Err(e) => {
            eprintln!("[portal] could not open RemoteDesktop session: {e}");
            None
        }
    }
}

/// A live libei sender session with a resumed keyboard device.
pub struct PortalPaster {
    state: Arc<Mutex<State>>,
    start: Instant,
}

struct State {
    connection: rev::Connection,
    keyboard: Option<rev::Device>,
    keymap: Option<SafeKeymap>,
    sequence: u32,
}

// xkb_keymap is documented thread-safe for reads; access is gated behind the
// outer State mutex regardless.
struct SafeKeymap(xkb::Keymap);
unsafe impl Send for SafeKeymap {}
unsafe impl Sync for SafeKeymap {}

#[derive(Clone, Copy)]
enum KeyDir {
    Press,
    Release,
    PressRelease,
}

/// A request handed to the libei worker: build a portal session, replying via
/// these std channels (usable from the caller's arbitrary thread).
struct BuildReq {
    init: mpsc::SyncSender<Result<Arc<Mutex<State>>, String>>,
    ready: mpsc::SyncSender<Result<(), String>>,
}

/// Handle to the single, long-lived libei worker thread. Kept process-global on
/// purpose: ashpd caches the D-Bus session connection in its own `OnceLock`, so
/// that connection must stay bound to a tokio runtime that lives for the whole
/// process. A per-attempt runtime would be torn down when the user cancels the
/// first consent dialog, killing the cached connection's I/O tasks — after
/// which every later attempt reuses a dead connection and the dialog never
/// reappears. One immortal worker runtime avoids that.
static WORKER: OnceLock<tokio::sync::mpsc::UnboundedSender<BuildReq>> = OnceLock::new();

fn worker() -> &'static tokio::sync::mpsc::UnboundedSender<BuildReq> {
    WORKER.get_or_init(|| {
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<BuildReq>();
        let spawned = std::thread::Builder::new()
            .name("clipboard-libei".into())
            .spawn(move || {
                let rt = match tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                {
                    Ok(rt) => rt,
                    Err(e) => {
                        eprintln!("[portal] failed to build libei runtime: {e}");
                        return;
                    }
                };
                // The static sender keeps this channel open forever, so the
                // runtime (and ashpd's connection bound to it) never dies.
                rt.block_on(async move {
                    while let Some(req) = rx.recv().await {
                        match open_context().await {
                            Ok(context) => {
                                // Sends init + ready, then drives the event
                                // stream until the session ends. This blocks the
                                // loop, which is fine: a live session is cached
                                // and never rebuilt. If it ever ends, we loop
                                // back and accept new build requests.
                                dispatcher(context, req.init, req.ready).await;
                            }
                            Err(e) => {
                                // Reply failure; `ready` drops so its waiter sees
                                // Disconnected. The runtime stays alive for the
                                // next attempt.
                                let _ = req.init.send(Err(e));
                            }
                        }
                    }
                });
            });
        if let Err(e) = spawned {
            eprintln!("[portal] failed to spawn libei worker thread: {e}");
        }
        tx
    })
}

impl PortalPaster {
    /// Build a live portal session, blocking the calling (paste/tray) thread.
    /// The request is handed to the single long-lived [`worker`] runtime so
    /// ashpd's process-global D-Bus connection stays valid across attempts (see
    /// WORKER). On first run this blocks through the consent dialog (bounded by
    /// `CONNECT_TIMEOUT_SECS`); later runs use the cached token and return fast.
    fn build_blocking() -> Result<Self, String> {
        // `init` carries the shared State once the libei handshake completes
        // (or an error from the portal / handshake); `ready` fires when the
        // first keyboard device resumes.
        let (init_tx, init_rx) = mpsc::sync_channel::<Result<Arc<Mutex<State>>, String>>(1);
        let (ready_tx, ready_rx) = mpsc::sync_channel::<Result<(), String>>(1);

        worker()
            .send(BuildReq {
                init: init_tx,
                ready: ready_tx,
            })
            .map_err(|_| "libei worker thread is not running".to_string())?;

        // Blocks through the consent dialog on first run.
        let state = match init_rx.recv_timeout(Duration::from_secs(CONNECT_TIMEOUT_SECS)) {
            Ok(res) => res?,
            Err(mpsc::RecvTimeoutError::Timeout) => {
                return Err(format!(
                    "timed out after {CONNECT_TIMEOUT_SECS}s negotiating the RemoteDesktop portal \
                     (consent dialog not answered, or the portal is unavailable)"
                ));
            }
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                return Err("libei thread exited before the portal session opened".into());
            }
        };

        // Wait for the first DeviceResumed carrying a keyboard.
        match ready_rx.recv_timeout(Duration::from_secs(READY_TIMEOUT_SECS)) {
            Ok(Ok(())) => {}
            Ok(Err(e)) => return Err(e),
            Err(mpsc::RecvTimeoutError::Timeout) => {
                return Err(format!(
                    "timed out after {READY_TIMEOUT_SECS}s waiting for a libei keyboard device"
                ));
            }
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                return Err("libei dispatcher ended before a device resumed".into());
            }
        }

        Ok(Self { state, start: Instant::now() })
    }

    fn timestamp_us(&self) -> u64 {
        self.start.elapsed().as_micros() as u64
    }

    /// Send the Ctrl+V chord. Ctrl stays logically held across frames until its
    /// explicit release, so the paste target sees a real Ctrl+V.
    fn send_ctrl_v(&self) -> Result<(), String> {
        self.key("Control_L", KeyDir::Press)?;
        self.key("v", KeyDir::PressRelease)?;
        self.key("Control_L", KeyDir::Release)?;
        Ok(())
    }

    fn key(&self, keysym: &str, dir: KeyDir) -> Result<(), String> {
        // Resolve keycodes under the lock, then DROP it before emit_key
        // re-locks (std Mutex isn't reentrant).
        let (keycode, needs_shift, shift_kc) = {
            let st = self.state.lock().unwrap();
            let keymap = st.keymap.as_ref().ok_or("no keymap received from portal")?;
            let (kc, needs_shift) = resolve_keycode(&keymap.0, keysym)
                .ok_or_else(|| format!("keysym '{keysym}' not found in server keymap"))?;
            let shift_kc = resolve_keycode(&keymap.0, "Shift_L").map(|(kc, _)| kc);
            (kc, needs_shift, shift_kc)
        };

        self.emit_key(|kb| {
            let press_shift = || {
                if needs_shift {
                    if let Some(kc) = shift_kc {
                        kb.key(kc, ei::keyboard::KeyState::Press);
                    }
                }
            };
            let release_shift = || {
                if needs_shift {
                    if let Some(kc) = shift_kc {
                        kb.key(kc, ei::keyboard::KeyState::Released);
                    }
                }
            };
            match dir {
                KeyDir::Press => {
                    press_shift();
                    kb.key(keycode, ei::keyboard::KeyState::Press);
                }
                KeyDir::Release => {
                    kb.key(keycode, ei::keyboard::KeyState::Released);
                    release_shift();
                }
                KeyDir::PressRelease => {
                    press_shift();
                    kb.key(keycode, ei::keyboard::KeyState::Press);
                    kb.key(keycode, ei::keyboard::KeyState::Released);
                    release_shift();
                }
            }
        })
    }

    /// Bracket one keyboard emulation frame: start_emulating → body → frame →
    /// stop_emulating → flush.
    fn emit_key<F>(&self, body: F) -> Result<(), String>
    where
        F: FnOnce(&ei::Keyboard),
    {
        let mut st = self.state.lock().unwrap();
        let serial = st.connection.serial();
        st.sequence = st.sequence.wrapping_add(1);
        let seq = st.sequence;

        let Some(device) = st.keyboard.clone() else {
            return Err("portal session has no keyboard device".into());
        };

        device.device().start_emulating(serial, seq);
        if let Some(kb) = device.interface::<ei::Keyboard>() {
            body(&kb);
        }
        device.device().frame(serial, self.timestamp_us());
        device.device().stop_emulating(serial);
        st.connection
            .flush()
            .map_err(|e| format!("ei flush failed: {e}"))?;
        Ok(())
    }
}

/// Runs on the libei worker runtime: handshake as sender, share `State` back
/// via `init_tx`, then drain events so pings get answered and devices resume.
/// Signals `ready_tx` once a keyboard device has resumed.
async fn dispatcher(
    context: ei::Context,
    init_tx: mpsc::SyncSender<Result<Arc<Mutex<State>>, String>>,
    ready_tx: mpsc::SyncSender<Result<(), String>>,
) {
    let (connection, mut stream) = match context
        .handshake_tokio(APP_ID, ei::handshake::ContextType::Sender)
        .await
    {
        Ok(pair) => pair,
        Err(err) => {
            let _ = init_tx.send(Err(format!("ei handshake failed: {err}")));
            return;
        }
    };
    let _ = connection.flush();

    let state = Arc::new(Mutex::new(State {
        connection,
        keyboard: None,
        keymap: None,
        sequence: 0,
    }));

    if init_tx.send(Ok(state.clone())).is_err() {
        return;
    }
    drop(init_tx);

    let mut ready_tx = Some(ready_tx);
    while let Some(result) = stream.next().await {
        let event = match result {
            Ok(ev) => ev,
            Err(err) => {
                eprintln!("[portal] libei stream error, ending dispatch: {err}");
                break;
            }
        };
        let mut st = state.lock().unwrap();
        if handle_event(&mut st, event) {
            if let Some(tx) = ready_tx.take() {
                let _ = tx.send(Ok(()));
            }
        }
    }
}

/// Open a libei context: honor `LIBEI_SOCKET` if set, otherwise negotiate one
/// through the RemoteDesktop portal (with restore-token caching).
async fn open_context() -> Result<ei::Context, String> {
    if let Ok(Some(context)) = ei::Context::connect_to_env() {
        return Ok(context);
    }

    let remote = RemoteDesktop::new().await.map_err(portal_err)?;
    let session = remote
        .create_session(CreateSessionOptions::default())
        .await
        .map_err(portal_err)?;

    // Try the cached token first. On any failure with a cached token, retry
    // once without it (forces a fresh consent dialog). Keep the old cache file
    // untouched if the retry also fails — the compositor might just be
    // hiccupping and next launch can try the token again.
    let cached_token = load_token();
    let used_cached = cached_token.is_some();

    // ashpd's Session has no Drop, so it must be closed explicitly on every
    // error path — otherwise a cancelled session lingers on the (process-global)
    // portal connection and can wedge later attempts.
    let selected = match run_session_flow(&remote, &session, cached_token.as_deref()).await {
        Ok(devices) => devices,
        Err(first_err) if used_cached => {
            eprintln!("[portal] cached token rejected ({first_err}); re-prompting for consent");
            match run_session_flow(&remote, &session, None).await {
                Ok(devices) => devices,
                Err(err) => {
                    let _ = session.close().await;
                    return Err(portal_err(err));
                }
            }
        }
        Err(err) => {
            let _ = session.close().await;
            return Err(portal_err(err));
        }
    };

    // Persist only when the portal actually returned a token (it may return
    // None if the user opted out of "remember this choice").
    if let Some(new_token) = selected.restore_token() {
        save_token(new_token);
    }

    let fd = match remote
        .connect_to_eis(&session, ConnectToEISOptions::default())
        .await
    {
        Ok(fd) => fd,
        Err(err) => {
            let _ = session.close().await;
            return Err(portal_err(err));
        }
    };
    // Success: deliberately do NOT close `session` — the EIS connection stays
    // valid only while the portal session is open. Dropping the handle (no Drop
    // impl) leaves it open, which is what we want.
    let stream = UnixStream::from(fd);
    match ei::Context::new(stream) {
        Ok(ctx) => Ok(ctx),
        Err(e) => {
            let _ = session.close().await;
            Err(format!("failed to create libei context: {e}"))
        }
    }
}

async fn run_session_flow(
    remote: &RemoteDesktop,
    session: &Session<RemoteDesktop>,
    restore_token: Option<&str>,
) -> ashpd::Result<SelectedDevices> {
    let devices: BitFlags<DeviceType> = DeviceType::Keyboard.into();
    let select_opts = SelectDevicesOptions::default()
        .set_devices(devices)
        .set_persist_mode(PersistMode::ExplicitlyRevoked)
        .set_restore_token(restore_token);
    remote
        .select_devices(session, select_opts)
        .await?
        .response()?;
    remote
        .start(session, None, StartOptions::default())
        .await?
        .response()
}

fn portal_err(e: ashpd::Error) -> String {
    format!(
        "RemoteDesktop portal error: {e}\n\
         The compositor may not expose org.freedesktop.portal.RemoteDesktop. Fix:\n  \
         GNOME: install xdg-desktop-portal-gnome\n  \
         KDE:   install xdg-desktop-portal-kde"
    )
}

/// Returns true once a keyboard device has resumed (i.e. we're ready to emit).
fn handle_event(st: &mut State, event: EiEvent) -> bool {
    match event {
        EiEvent::SeatAdded(ev) => {
            // Keyboard-only: bind just the keyboard capability. (This also
            // sidesteps the KWin bug where binding Keyboard alongside Pointer
            // in one call silently drops the keyboard device.)
            let caps: BitFlags<DeviceCapability> = DeviceCapability::Keyboard.into();
            ev.seat.bind_capabilities(caps);
            let _ = st.connection.flush();
        }
        EiEvent::DeviceAdded(ev) => {
            let device = ev.device.clone();
            if device.has_capability(DeviceCapability::Keyboard) {
                if let Some(keymap_info) = device.keymap() {
                    match load_keymap(keymap_info) {
                        Ok(km) => st.keymap = Some(SafeKeymap(km)),
                        Err(err) => eprintln!("[portal] failed to load keymap from EIS: {err}"),
                    }
                }
                st.keyboard = Some(device);
            }
        }
        EiEvent::DeviceResumed(_) => {
            return st.keyboard.is_some();
        }
        EiEvent::DeviceRemoved(ev) => {
            if st.keyboard.as_ref() == Some(&ev.device) {
                st.keyboard = None;
            }
        }
        _ => {}
    }
    false
}

fn load_keymap(km: &rev::Keymap) -> Result<xkb::Keymap, String> {
    let fd = km.fd.try_clone().map_err(|e| format!("keymap fd clone: {e}"))?;
    let ctx = xkb::Context::new(xkb::CONTEXT_NO_FLAGS);
    let keymap = unsafe {
        xkb::Keymap::new_from_fd(
            &ctx,
            fd,
            km.size as usize,
            xkb::KEYMAP_FORMAT_TEXT_V1,
            xkb::KEYMAP_COMPILE_NO_FLAGS,
        )
    }
    .map_err(|e| format!("xkb keymap compile: {e}"))?
    .ok_or("xkb_keymap_new_from_fd returned null")?;
    Ok(keymap)
}

/// Find the evdev keycode that produces `name` at shift level 0 or 1.
/// Returns `(evdev_keycode, needs_shift)`. Evdev codes are xkb codes minus 8.
fn resolve_keycode(keymap: &xkb::Keymap, name: &str) -> Option<(u32, bool)> {
    let target = xkb::keysym_from_name(name, xkb::KEYSYM_NO_FLAGS);
    if target.raw() == 0 {
        return None;
    }
    for keycode in keymap.min_keycode().raw()..=keymap.max_keycode().raw() {
        for level in 0..=1 {
            let syms = keymap.key_get_syms_by_level(xkb::Keycode::new(keycode), 0, level);
            if syms.contains(&target) {
                return Some((keycode.saturating_sub(8), level == 1));
            }
        }
    }
    None
}

// --- restore_token cache -------------------------------------------------
//
// Best-effort JSON at `$XDG_STATE_HOME/linux-clipboard/portal.token` (mode
// 0600). Any read failure degrades to "no token" (first-run consent flow).

#[derive(Serialize, Deserialize)]
struct CachedToken {
    schema_version: u32,
    token: String,
}

const TOKEN_SCHEMA_VERSION: u32 = 1;

fn token_path() -> Option<PathBuf> {
    let base = std::env::var_os("XDG_STATE_HOME")
        .map(PathBuf::from)
        .filter(|p| p.is_absolute())
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".local/state")))?;
    Some(base.join("linux-clipboard").join("portal.token"))
}

fn load_token() -> Option<String> {
    let path = token_path()?;
    let contents = std::fs::read_to_string(&path).ok()?;
    let cached: CachedToken = serde_json::from_str(&contents).ok()?;
    (cached.schema_version == TOKEN_SCHEMA_VERSION).then_some(cached.token)
}

fn save_token(token: &str) {
    let Some(path) = token_path() else { return };
    if let Some(parent) = path.parent() {
        let _ = std::fs::DirBuilder::new()
            .recursive(true)
            .mode(0o700)
            .create(parent);
    }
    let payload = match serde_json::to_vec_pretty(&CachedToken {
        schema_version: TOKEN_SCHEMA_VERSION,
        token: token.to_owned(),
    }) {
        Ok(p) => p,
        Err(_) => return,
    };

    // Write to a pid-suffixed tmp file at mode 0600, then rename (atomic on the
    // same filesystem) so a reader never sees a half-written token.
    let tmp = path.with_file_name(format!("portal.token.tmp.{}", std::process::id()));
    let write = (|| -> std::io::Result<()> {
        let mut f = std::fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .mode(0o600)
            .open(&tmp)?;
        f.write_all(&payload)?;
        let _ = f.sync_all();
        Ok(())
    })();
    if write.is_err() {
        let _ = std::fs::remove_file(&tmp);
        return;
    }
    if std::fs::rename(&tmp, &path).is_err() {
        let _ = std::fs::remove_file(&tmp);
    }
}
