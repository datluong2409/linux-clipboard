//! Global hotkey registration + runtime rebind (X11 only; the plugin silently
//! no-ops on Wayland, which is why the GNOME-shortcut fallback exists).

use crate::state::AppState;
use crate::window;
use std::str::FromStr;
use tauri::{AppHandle, Manager};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutState};

pub fn register(app: &AppHandle, accel: &str) -> Result<(), String> {
    let shortcut = Shortcut::from_str(accel).map_err(|_| "invalid".to_string())?;
    let handler_app = app.clone();
    app.global_shortcut()
        .on_shortcut(shortcut, move |_app, _sc, event| {
            if event.state == ShortcutState::Pressed {
                window::toggle(&handler_app);
            }
        })
        .map_err(|e| e.to_string())?;

    if let Some(state) = app.try_state::<AppState>() {
        if let Ok(mut g) = state.current_hotkey.lock() {
            *g = Some(accel.to_string());
        }
    }
    Ok(())
}

pub fn unregister(app: &AppHandle, accel: &str) {
    if let Ok(sc) = Shortcut::from_str(accel) {
        let _ = app.global_shortcut().unregister(sc);
    }
}

/// Unregister the previous hotkey and register a new one, rolling back on error.
pub fn rebind(app: &AppHandle, new_accel: &str) -> Result<(), String> {
    // Reject clearly-invalid accelerators before touching the live binding.
    Shortcut::from_str(new_accel).map_err(|_| "invalid".to_string())?;

    let old = app
        .try_state::<AppState>()
        .and_then(|s| s.current_hotkey.lock().ok().and_then(|g| g.clone()));

    if let Some(old) = &old {
        unregister(app, old);
    }
    match register(app, new_accel) {
        Ok(()) => Ok(()),
        Err(e) => {
            if let Some(old) = &old {
                let _ = register(app, old);
            }
            Err(e)
        }
    }
}
