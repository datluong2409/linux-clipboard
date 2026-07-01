//! Detect the display server and probe which input mechanisms are usable.
//!
//! - `tauri-plugin-global-shortcut` only works under X11.
//! - Auto-paste uses `enigo` on X11; on Wayland it needs the external
//!   `ydotool` daemon, otherwise we degrade to copy-only.

use crate::models::SessionInfo;

pub fn detect() -> SessionInfo {
    let kind = match std::env::var("XDG_SESSION_TYPE").ok().as_deref() {
        Some("x11") => "x11",
        Some("wayland") => "wayland",
        _ => {
            if std::env::var("WAYLAND_DISPLAY").is_ok() {
                "wayland"
            } else if std::env::var("DISPLAY").is_ok() {
                "x11"
            } else {
                "unknown"
            }
        }
    }
    .to_string();

    let is_gnome = std::env::var("XDG_CURRENT_DESKTOP")
        .map(|d| d.to_lowercase().contains("gnome"))
        .unwrap_or(false);

    let can_global_shortcut = kind == "x11";

    let (can_auto_paste, backend) = if kind == "x11" {
        (true, "enigo")
    } else if bin_on_path("ydotool") {
        (true, "ydotool")
    } else {
        (false, "none")
    };

    SessionInfo {
        kind,
        is_gnome,
        can_global_shortcut,
        can_auto_paste,
        auto_paste_backend: backend.to_string(),
    }
}

/// True if `bin` is an executable file somewhere on `$PATH`.
pub fn bin_on_path(bin: &str) -> bool {
    std::env::var_os("PATH")
        .map(|paths| std::env::split_paths(&paths).any(|dir| dir.join(bin).is_file()))
        .unwrap_or(false)
}
