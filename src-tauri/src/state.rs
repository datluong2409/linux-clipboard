//! Shared application state (managed by Tauri; `Send + Sync`).

use crate::models::{SessionInfo, Settings};
use rusqlite::Connection;
use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::sync::{Mutex, RwLock};
use std::time::{Duration, Instant};

/// How long after a programmatic clipboard write we ignore clipboard changes,
/// so our own paste-back write isn't recorded as a new history entry.
const SUPPRESS_WINDOW: Duration = Duration::from_millis(1200);

pub struct AppState {
    pub db: Mutex<Connection>,
    pub settings: RwLock<Settings>,
    pub session: SessionInfo,
    pub images_dir: PathBuf,
    pub config_path: PathBuf,

    /// Hash of the last clipboard content we observed (change detection).
    pub last_seen_hash: Mutex<Option<String>>,
    /// When set (and not yet elapsed), the next observed change is treated as
    /// our own write and skipped.
    pub suppress_until: Mutex<Option<Instant>>,
    /// Accelerator string currently registered with the global-shortcut plugin.
    pub current_hotkey: Mutex<Option<String>>,
    /// Pause flag for the monitor (unused for now; handy for tests / settings).
    pub monitor_paused: AtomicBool,

    /// Lazily-built Wayland paste backend (XDG RemoteDesktop portal + libei).
    /// Built on the first auto-paste; reused afterwards. See `portal.rs`.
    pub paste_backend: crate::portal::PortalCell,

    /// Set once we've shown the "enable auto-paste?" prompt this session, so the
    /// user isn't nagged on every paste while the portal permission is ungranted.
    pub paste_prompt_shown: AtomicBool,
}

impl AppState {
    pub fn settings(&self) -> Settings {
        self.settings.read().map(|s| s.clone()).unwrap_or_default()
    }

    /// The user's chosen UI language, for localizing backend-rendered text
    /// (tray menu, native dialogs).
    pub fn lang(&self) -> crate::i18n::Lang {
        crate::i18n::Lang::from_code(&self.settings().language)
    }

    /// Arm suppression before a programmatic clipboard write.
    pub fn arm_suppress(&self) {
        if let Ok(mut guard) = self.suppress_until.lock() {
            *guard = Some(Instant::now() + SUPPRESS_WINDOW);
        }
    }

    /// Returns true (and clears the flag) if suppression is currently active.
    pub fn consume_suppress(&self) -> bool {
        if let Ok(mut guard) = self.suppress_until.lock() {
            let active = matches!(*guard, Some(t) if Instant::now() < t);
            *guard = None;
            active
        } else {
            false
        }
    }

    pub fn set_last_seen(&self, hash: Option<String>) {
        if let Ok(mut guard) = self.last_seen_hash.lock() {
            *guard = hash;
        }
    }

    pub fn last_seen(&self) -> Option<String> {
        self.last_seen_hash.lock().ok().and_then(|g| g.clone())
    }
}
