//! The `#[tauri::command]` surface invoked from the React frontend.

use crate::models::{Clip, OpResult, SessionInfo, Settings, UpdateCheck};
use crate::state::AppState;
use crate::{clipboard, db, gnome, hotkey, images, settings, updater, window};
use tauri::{AppHandle, Emitter, Manager, State};
use tauri_plugin_dialog::{DialogExt, MessageDialogButtons};

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
        Ok(conn) => {
            let clip = db::get(&conn, id);
            // Picking an item counts as re-using it: bump it to the top of the
            // list (like re-copying existing content does). The paste-back write
            // below is suppressed from the monitor, so it wouldn't bump on its own.
            if clip.is_some() {
                db::bump_used(&conn, id);
            }
            clip
        }
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

    // 4. Hide our window first so focus returns to the target app.
    window::hide_panel(&app);
    let _ = app.emit("history-updated", ());

    // 5. Auto-paste, or fall back to copy-only.
    if !(cfg.auto_paste && st.session.can_auto_paste) {
        return OpResult {
            ok: true,
            reason: Some("copied".into()),
        };
    }

    // On Wayland the RemoteDesktop portal permission may not be granted yet.
    // Rather than silently popping the OS consent mid-paste, ask the user once
    // per session whether to enable it. The clip is already on the clipboard,
    // so declining just means copy-only (manual Ctrl+V).
    if st.session.auto_paste_backend == "portal" && !crate::portal::is_granted(&st.paste_backend) {
        use std::sync::atomic::Ordering;
        if st
            .paste_prompt_shown
            .compare_exchange(false, true, Ordering::Relaxed, Ordering::Relaxed)
            .is_ok()
        {
            prompt_enable_paste(app.clone());
        }
        return OpResult {
            ok: true,
            reason: Some("copied".into()),
        };
    }

    // Granted (or X11): auto-paste after a short focus-settle delay.
    let session = st.session.clone();
    let portal = st.paste_backend.clone();
    std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_millis(140));
        crate::paste::paste(&session, &portal);
    });
    OpResult::ok()
}

/// Ask (once per session) whether to grant the Wayland RemoteDesktop permission
/// so auto-paste works. Runs off-thread because both the dialog and the consent
/// flow block. The clip has already been copied by the caller, so declining is
/// a safe copy-only fallback.
fn prompt_enable_paste(app: AppHandle) {
    std::thread::spawn(move || {
        let tr = app.state::<AppState>().lang();

        // No RemoteDesktop portal backend installed → auto-paste is impossible;
        // warn with install instructions instead of offering to enable.
        if !crate::portal::remote_desktop_available() {
            let _ = app
                .dialog()
                .message(format!("{}\n\n{}", tr.portal_missing_title(), tr.portal_missing_body()))
                .title(tr.app_title())
                .blocking_show();
            return;
        }

        let enable = app
            .dialog()
            .message(format!("{}\n\n{}", tr.enable_paste_title(), tr.enable_paste_body()))
            .title(tr.app_title())
            .buttons(MessageDialogButtons::OkCancelCustom(
                tr.enable_paste_now().into(),
                tr.enable_paste_later().into(),
            ))
            .blocking_show();

        if enable {
            let cell = app.state::<AppState>().paste_backend.clone();
            let _ = crate::portal::enable(&cell);
            let app_ui = app.clone();
            let _ = app.run_on_main_thread(move || crate::tray::refresh(&app_ui));
        } else {
            let _ = app
                .dialog()
                .message(format!("{}\n\n{}", tr.copied_title(), tr.copied_body()))
                .title(tr.app_title())
                .blocking_show();
        }
    });
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
    if settings.auto_paste != old.auto_paste || settings.language != old.language {
        // Rebuild the tray menu so its auto-paste label and (on a language
        // change) all its item labels reflect the new settings.
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

/// Turn auto-paste on/off from the Settings UI, running the exact same state
/// machine as the tray toggle (grant flow on Wayland, portal-missing warning).
#[tauri::command]
pub fn set_auto_paste(app: AppHandle, enabled: bool) {
    crate::tray::apply_auto_paste(&app, enabled);
}

/// The live auto-paste state for the Settings UI:
/// "on" | "off" | "needs_permission" | "portal_missing".
#[tauri::command]
pub fn get_paste_state(state: State<'_, AppState>) -> String {
    crate::tray::auto_paste_status(&state).to_string()
}

/// The running app version (from tauri.conf.json / Cargo.toml), shown in the
/// Settings "Updates" section.
#[tauri::command]
pub fn get_version(app: AppHandle) -> String {
    app.package_info().version.to_string()
}

/// Ask GitHub whether a newer release exists. Runs the blocking HTTPS GET off
/// the async runtime so the UI stays responsive while the network call is in
/// flight.
#[tauri::command]
pub async fn check_for_updates(app: AppHandle) -> UpdateCheck {
    let current = app.package_info().version.to_string();
    let fallback = current.clone();
    tauri::async_runtime::spawn_blocking(move || updater::check(&current))
        .await
        .unwrap_or_else(|_| UpdateCheck::failed(fallback, "internal"))
}

/// Open a release URL (or the latest-release page) in the user's browser so
/// they can download the new `.deb` / AppImage.
#[tauri::command]
pub fn open_release_page(url: Option<String>) {
    let target = url.unwrap_or_else(updater::releases_page);
    updater::open_url(&target);
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

/// The exact `<app> --toggle` command a user should bind to a key in their
/// desktop's own keyboard settings. Surfaced by the Settings UI on sessions
/// with no automatic hotkey backend (e.g. non-GNOME Wayland) so it can be shown
/// and copied. Falls back to the bare binary name if the exe path can't be
/// resolved, so the UI always has something to display.
#[tauri::command]
pub fn get_toggle_command() -> String {
    gnome_toggle_command().unwrap_or_else(|_| "linux-clipboard --toggle".into())
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

/// Install the panel hotkey at startup per the detected backend. Emits a
/// `[hotkey]` line on stderr for every outcome so a user who launches the app
/// from a terminal can see whether the shortcut was set up (and why not).
pub fn init_hotkey(app: &AppHandle) {
    let Some(st) = app.try_state::<AppState>() else {
        return;
    };
    let backend = st.session.hotkey_backend.clone();
    let accel = st.settings().hotkey;
    match backend.as_str() {
        // GNOME (X11 or Wayland): the custom keybinding lives in gsettings and
        // persists across launches, so create it only when missing — don't
        // overwrite a combo the user may have retuned in GNOME's own settings.
        // (A rebind from our Settings UI still force-overwrites via set_hotkey.)
        "gnome" => {
            let command = match gnome_toggle_command() {
                Ok(c) => c,
                Err(e) => {
                    eprintln!("[hotkey] cannot resolve toggle command: {e}");
                    return;
                }
            };
            match gnome::ensure(&command, &gnome::to_gnome_accel(&accel)) {
                Ok(created) => {
                    if created {
                        eprintln!("[hotkey] created GNOME custom shortcut for {accel}");
                    } else {
                        eprintln!("[hotkey] GNOME custom shortcut already present, leaving as-is");
                    }
                    if let Ok(mut g) = st.settings.write() {
                        g.gnome_shortcut_configured = true;
                    }
                    let _ = settings::save(&st.config_path, &st.settings());
                }
                Err(e) => eprintln!("[hotkey] failed to create GNOME custom shortcut: {e}"),
            }
        }
        // Non-GNOME X11: the in-app global-shortcut plugin registers per-process,
        // so it must be (re)bound on every launch.
        "global-shortcut" => {
            if let Err(e) = hotkey::rebind(app, &accel) {
                eprintln!("[hotkey] failed to register global shortcut {accel}: {e}");
            }
        }
        // No automatic trigger (e.g. non-GNOME Wayland): the user binds it by hand.
        _ => eprintln!(
            "[hotkey] no automatic hotkey backend for this session; bind {accel} manually to `<app> --toggle`"
        ),
    }
}
