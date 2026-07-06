//! Serde types shared with the frontend (camelCase over the IPC boundary).

use serde::{Deserialize, Serialize};

/// One clipboard entry. Text lives in `content`; images live on disk and are
/// referenced by `image_path` / `thumb_path` (absolute paths the frontend turns
/// into asset URLs via `convertFileSrc`).
#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Clip {
    pub id: i64,
    pub kind: String, // "text" | "image"
    pub content: Option<String>,
    /// For rich-text clips: the captured `text/html`, replayed on paste so
    /// formatting (bold/italic/underline, links, colors) survives. `None` for
    /// plain text and images.
    pub html: Option<String>,
    pub image_path: Option<String>,
    pub thumb_path: Option<String>,
    pub width: Option<i64>,
    pub height: Option<i64>,
    pub byte_size: Option<i64>,
    pub pinned: bool,
    pub created_at: i64,
    pub last_used_at: i64,
}

/// Persisted user settings (stored as JSON in the app config dir).
#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    /// Tauri accelerator string, e.g. "Alt+V".
    pub hotkey: String,
    pub auto_paste: bool,
    pub history_cap: u32,
    pub max_image_bytes: u64,
    /// "cursor" (X11) | "center" (Wayland / forced).
    pub position_mode: String,
    /// "system" | "light" | "dark".
    pub theme: String,
    pub gnome_shortcut_configured: bool,
    pub first_run_done: bool,
    /// UI language: "en" | "vi". `#[serde(default)]` keeps older settings.json
    /// files (written before this field existed) loadable instead of resetting.
    #[serde(default = "default_language")]
    pub language: String,
}

fn default_language() -> String {
    "en".into()
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            hotkey: "Alt+V".into(),
            auto_paste: true,
            history_cap: 25,
            max_image_bytes: 5 * 1024 * 1024,
            position_mode: "cursor".into(),
            theme: "system".into(),
            gnome_shortcut_configured: false,
            first_run_done: false,
            language: default_language(),
        }
    }
}

/// Session / capability info surfaced to the Settings UI so it can show the
/// right hotkey mechanism and explain any degraded behavior.
#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct SessionInfo {
    pub kind: String, // "x11" | "wayland" | "unknown"
    pub is_gnome: bool,
    pub can_global_shortcut: bool,
    /// Which mechanism triggers the panel hotkey in this session:
    /// "gnome" (GNOME custom keybinding via gsettings, X11 or Wayland) |
    /// "global-shortcut" (in-app plugin, non-GNOME X11) |
    /// "none" (no automatic trigger, e.g. non-GNOME Wayland).
    pub hotkey_backend: String,
    pub can_auto_paste: bool,
    pub auto_paste_backend: String, // "enigo" (X11) | "portal" (Wayland libei) | "none"
}

/// Result of a GitHub-Releases update check, surfaced to the tray + Settings UI.
/// `error` is `None` on success (including "no release published yet", which
/// reports as up-to-date); a stable code otherwise: "network" | "parse" |
/// "internal".
#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct UpdateCheck {
    pub current_version: String,
    pub latest_version: Option<String>,
    pub update_available: bool,
    pub release_url: Option<String>,
    pub error: Option<String>,
}

impl UpdateCheck {
    /// A failed check (couldn't reach GitHub / parse the response).
    pub fn failed(current: String, error: &str) -> Self {
        Self {
            current_version: current,
            latest_version: None,
            update_available: false,
            release_url: None,
            error: Some(error.into()),
        }
    }
}

/// Generic result for operations the frontend wants to react to (e.g. rebind).
#[derive(Serialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct OpResult {
    pub ok: bool,
    pub reason: Option<String>,
}

impl OpResult {
    pub fn ok() -> Self {
        Self { ok: true, reason: None }
    }
    pub fn err(reason: impl Into<String>) -> Self {
        Self { ok: false, reason: Some(reason.into()) }
    }
}
