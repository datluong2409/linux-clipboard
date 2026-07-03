//! Run-on-login toggle (writes a .desktop file under ~/.config/autostart on Linux).

use tauri::AppHandle;
use tauri_plugin_autostart::ManagerExt;

pub fn set(app: &AppHandle, enabled: bool) -> Result<(), String> {
    let mgr = app.autolaunch();
    let res = if enabled { mgr.enable() } else { mgr.disable() };
    res.map_err(|e| e.to_string())
}
