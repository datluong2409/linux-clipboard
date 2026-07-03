//! Synthetic Ctrl+V (auto-paste) and cursor location, via enigo (X11) or the
//! XDG RemoteDesktop portal + libei (Wayland, see `portal.rs`). Never errors
//! hard: returns false when no backend is available so callers can fall back
//! to copy-only.

use crate::models::SessionInfo;
use crate::portal::PortalCell;
use enigo::{Direction, Enigo, Key, Keyboard, Mouse, Settings as EnigoSettings};
use std::time::Duration;

/// Simulate Ctrl+V with the best available backend. Returns true if a
/// keystroke was actually dispatched. `portal` is the lazily-built Wayland
/// paste session (ignored on X11).
pub fn paste(session: &SessionInfo, portal: &PortalCell) -> bool {
    match session.auto_paste_backend.as_str() {
        "enigo" => paste_enigo(),
        "portal" => crate::portal::paste_ctrl_v(portal),
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

/// Current mouse position in physical pixels (X11). Used to place the panel
/// near the cursor like the Windows Win+V flyout.
pub fn cursor_location() -> Option<(i32, i32)> {
    let enigo = Enigo::new(&EnigoSettings::default()).ok()?;
    enigo.location().ok()
}
