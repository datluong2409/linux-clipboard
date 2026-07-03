//! The `#[tauri::command]` surface invoked from the React frontend.

use crate::models::{Clip, OpResult, SessionInfo, Settings};
use crate::state::AppState;
use crate::{autostart, clipboard, db, gnome, hotkey, images, settings, window};
use tauri::{AppHandle, Emitter, Manager, State};

#[tauri::command]
pub fn get_history(state: State<'_, AppState>, limit: Option<i64>) -> Vec<Clip> {
    let limit = limit.unwrap_or(200);
    state
        .db
        .lock()
        .map(|c| db::list_history(&c, limit))
        .unwrap_or_default()
}

#[tauri::command]
pub fn search_history(state: State<'_, AppState>, query: String, limit: Option<i64>) -> Vec<Clip> {
    let limit = limit.unwrap_or(200);
    let Ok(conn) = state.db.lock() else {
        return Vec::new();
    };
    if query.trim().is_empty() {
        db::list_history(&conn, limit)
    } else {
        db::search(&conn, &query, limit)
    }
}

#[tauri::command]
pub fn get_pins(state: State<'_, AppState>) -> Vec<Clip> {
    state.db.lock().map(|c| db::list_pins(&c)).unwrap_or_default()
}

#[tauri::command]
pub fn pin_item(app: AppHandle, state: State<'_, AppState>, id: i64, pinned: bool) {
    if let Ok(conn) = state.db.lock() {
        let _ = db::set_pinned(&conn, id, pinned);
    }
    let _ = app.emit("history-updated", ());
}

#[tauri::command]
pub fn delete_item(app: AppHandle, state: State<'_, AppState>, id: i64) {
    let gc = state.db.lock().map(|c| db::delete(&c, id)).unwrap_or_default();
    images::delete_files(&gc);
    let _ = app.emit("history-updated", ());
}

#[tauri::command]
pub fn clear_history(app: AppHandle, state: State<'_, AppState>, keep_pinned: bool) {
    let gc = state
        .db
        .lock()
        .map(|c| db::clear(&c, keep_pinned))
        .unwrap_or_default();
    images::delete_files(&gc);
    let _ = app.emit("history-updated", ());
}

#[tauri::command]
pub fn paste_item(app: AppHandle, state: State<'_, AppState>, id: i64) -> OpResult {
    let st = state.inner();
    let clip = match st.db.lock() {
        Ok(conn) => db::get(&conn, id),
        Err(_) => return OpResult::err("db_lock"),
    };
    let Some(clip) = clip else {
        return OpResult::err("not_found");
    };

    // 1-2. Put the item back on the clipboard (arms suppression internally).
    match clip.kind.as_str() {
        "text" => {
            if let Some(text) = clip.content.clone() {
                clipboard::write_text(st, text);
            }
        }
        "image" => {
            if let Some(path) = clip.image_path.clone() {
                clipboard::write_image_from_path(st, &path);
            }
        }
        _ => {}
    }

    let cfg = st.settings();
    let do_paste = cfg.auto_paste && st.session.can_auto_paste;

    // 4. Hide our window first so focus returns to the target app.
    window::hide_panel(&app);
    let _ = app.emit("history-updated", ());

    // 5. Auto-paste after a short focus-settle delay, or fall back to copy-only.
    if do_paste {
        let session = st.session.clone();
        let portal = st.paste_backend.clone();
        std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_millis(140));
            crate::paste::paste(&session, &portal);
        });
        OpResult::ok()
    } else {
        OpResult {
            ok: true,
            reason: Some("copied".into()),
        }
    }
}

#[tauri::command]
pub fn toggle_panel(app: AppHandle) {
    window::toggle(&app);
}

#[tauri::command]
pub fn hide_panel(app: AppHandle) {
    window::hide_panel(&app);
}

#[tauri::command]
pub fn get_settings(state: State<'_, AppState>) -> Settings {
    state.settings()
}

#[tauri::command]
pub fn set_settings(app: AppHandle, state: State<'_, AppState>, settings: Settings) -> OpResult {
    let st = state.inner();
    let old = st.settings();
    if let Ok(mut g) = st.settings.write() {
        *g = settings.clone();
    }
    let _ = crate::settings::save(&st.config_path, &settings);

    if settings.hotkey != old.hotkey {
        let _ = apply_hotkey(&app, &st.session.hotkey_backend, &settings.hotkey);
    }
    if settings.autostart != old.autostart {
        let _ = autostart::set(&app, settings.autostart);
    }
    if settings.auto_paste != old.auto_paste {
        // Keep the tray's auto-paste toggle label in sync with the Settings UI.
        let app2 = app.clone();
        let _ = app.run_on_main_thread(move || crate::tray::refresh(&app2));
    }
    let _ = app.emit("settings-updated", &settings);
    OpResult::ok()
}

#[tauri::command]
pub fn set_hotkey(app: AppHandle, state: State<'_, AppState>, accel: String) -> OpResult {
    let st = state.inner();
    let backend = st.session.hotkey_backend.clone();
    match apply_hotkey(&app, &backend, &accel) {
        Ok(()) => {
            if let Ok(mut g) = st.settings.write() {
                g.hotkey = accel.clone();
                if backend == "gnome" {
                    g.gnome_shortcut_configured = true;
                }
            }
            let _ = settings::save(&st.config_path, &st.settings());
            OpResult::ok()
        }
        Err(e) => {
            // No automatic backend (e.g. non-GNOME Wayland): still remember the
            // chosen combo so the UI shows it and the user can bind it manually.
            if e == "no_hotkey_backend" {
                if let Ok(mut g) = st.settings.write() {
                    g.hotkey = accel.clone();
                }
                let _ = settings::save(&st.config_path, &st.settings());
            }
            OpResult::err(e)
        }
    }
}

#[tauri::command]
pub fn get_session_info(state: State<'_, AppState>) -> SessionInfo {
    state.session.clone()
}

/// The shell command a GNOME custom keybinding runs to toggle the panel.
/// (`tauri-plugin-single-instance` forwards `--toggle` to the running app.)
fn gnome_toggle_command() -> Result<String, String> {
    let exe = std::env::current_exe()
        .map(|p| p.to_string_lossy().into_owned())
        .unwrap_or_default();
    if exe.is_empty() {
        return Err("no_exe".into());
    }
    Ok(format!("{exe} --toggle"))
}

/// Bind `accel` using whichever hotkey backend this session supports. Callers
/// own persisting the accelerator into settings.
fn apply_hotkey(app: &AppHandle, backend: &str, accel: &str) -> Result<(), String> {
    match backend {
        // GNOME (X11 or Wayland): (re)write our dedicated custom keybinding,
        // overwriting any previous value — this is the "sync with GNOME" path.
        "gnome" => {
            let command = gnome_toggle_command()?;
            gnome::configure(&command, &gnome::to_gnome_accel(accel))
        }
        // Non-GNOME X11: register/rebind the in-app global shortcut plugin.
        "global-shortcut" => hotkey::rebind(app, accel),
        _ => Err("no_hotkey_backend".into()),
    }
}

/// Install the panel hotkey at startup per the detected backend: (re)sync the
/// GNOME custom keybinding, or register the in-app global shortcut. No-op when
/// there is no automatic backend for this session.
pub fn init_hotkey(app: &AppHandle) {
    let Some(st) = app.try_state::<AppState>() else {
        return;
    };
    let backend = st.session.hotkey_backend.clone();
    let accel = st.settings().hotkey;
    if apply_hotkey(app, &backend, &accel).is_ok() && backend == "gnome" {
        if let Ok(mut g) = st.settings.write() {
            g.gnome_shortcut_configured = true;
        }
        let _ = settings::save(&st.config_path, &st.settings());
    }
}

#[tauri::command]
pub fn set_autostart(app: AppHandle, state: State<'_, AppState>, enabled: bool) -> OpResult {
    match autostart::set(&app, enabled) {
        Ok(()) => {
            let st = state.inner();
            if let Ok(mut g) = st.settings.write() {
                g.autostart = enabled;
            }
            let _ = settings::save(&st.config_path, &st.settings());
            OpResult::ok()
        }
        Err(e) => OpResult::err(e),
    }
}
