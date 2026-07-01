//! Synthetic Ctrl+V (auto-paste) and cursor location, via enigo (X11) or the
//! external ydotool daemon (Wayland). Never errors hard: returns false when no
//! backend is available so callers can fall back to copy-only.

use crate::models::SessionInfo;
use enigo::{Direction, Enigo, Key, Keyboard, Mouse, Settings as EnigoSettings};
use std::process::Command;
use std::time::Duration;

/// Simulate Ctrl+V with the best available backend. Returns true if a
/// keystroke was actually dispatched.
pub fn paste(session: &SessionInfo) -> bool {
    match session.auto_paste_backend.as_str() {
        "enigo" => paste_enigo(),
        "ydotool" => paste_ydotool(),
        _ => false,
    }
}

fn paste_enigo() -> bool {
    let Ok(mut enigo) = Enigo::new(&EnigoSettings::default()) else {
        return false;
    };
    let mut ok = enigo.key(Key::Control, Direction::Press).is_ok();
    std::thread::sleep(Duration::from_millis(20));
    ok &= enigo.key(Key::Unicode('v'), Direction::Click).is_ok();
    std::thread::sleep(Duration::from_millis(20));
    ok &= enigo.key(Key::Control, Direction::Release).is_ok();
    ok
}

fn paste_ydotool() -> bool {
    // Linux input event codes: 29 = LEFTCTRL, 47 = V. ":1" press, ":0" release.
    Command::new("ydotool")
        .args(["key", "29:1", "47:1", "47:0", "29:0"])
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

/// Current mouse position in physical pixels (X11). Used to place the panel
/// near the cursor like the Windows Win+V flyout.
pub fn cursor_location() -> Option<(i32, i32)> {
    let enigo = Enigo::new(&EnigoSettings::default()).ok()?;
    enigo.location().ok()
}
