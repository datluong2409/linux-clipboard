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
    /// Tauri accelerator string, e.g. "Ctrl+Alt+V".
    pub hotkey: String,
    pub auto_paste: bool,
    pub autostart: bool,
    pub history_cap: u32,
    pub capture_images: bool,
    pub max_image_bytes: u64,
    pub hide_on_blur: bool,
    /// "cursor" (X11) | "center" (Wayland / forced).
    pub position_mode: String,
    /// "system" | "light" | "dark".
    pub theme: String,
    pub gnome_shortcut_configured: bool,
    pub first_run_done: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            hotkey: "Ctrl+Alt+V".into(),
            auto_paste: true,
            autostart: false,
            history_cap: 50,
            capture_images: true,
            max_image_bytes: 5 * 1024 * 1024,
            hide_on_blur: true,
            position_mode: "cursor".into(),
            theme: "system".into(),
            gnome_shortcut_configured: false,
            first_run_done: false,
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
    pub can_auto_paste: bool,
    pub auto_paste_backend: String, // "enigo" | "ydotool" | "none"
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
