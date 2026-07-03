mod autostart;
mod clipboard;
mod commands;
mod db;
mod gnome;
mod hotkey;
mod images;
mod models;
mod paste;
mod portal;
mod session;
mod settings;
mod state;
mod tray;
mod util;
mod window;

use state::AppState;
use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::sync::{Mutex, RwLock};
use tauri::Manager;
use tauri_plugin_autostart::MacosLauncher;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        // single-instance must be registered first; forwards `--toggle` from a
        // GNOME custom shortcut (or a second launch) to the running instance.
        .plugin(tauri_plugin_single_instance::init(|app, argv, _cwd| {
            if argv.iter().any(|a| a.as_str() == "--toggle") {
                window::toggle(app);
            } else {
                window::show_panel(app);
            }
        }))
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_autostart::init(
            MacosLauncher::LaunchAgent,
            Some(vec!["--hidden"]),
        ))
        .setup(|app| {
            let handle = app.handle().clone();

            let data_dir = app.path().app_data_dir()?;
            let config_dir = app.path().app_config_dir()?;
            let images_dir = data_dir.join("images");
            let config_path = config_dir.join("settings.json");
            let _ = std::fs::create_dir_all(&images_dir);
            let _ = std::fs::create_dir_all(&config_dir);

            let loaded = settings::load(&config_path);
            let db_path = data_dir.join("history.db");
            let conn = db::open(&db_path).expect("failed to open history database");
            let session = session::detect();

            // Reflect the real OS autostart state into settings.
            let mut settings_val = loaded;
            settings_val.autostart = autostart::is_enabled(&handle);

            app.manage(AppState {
                db: Mutex::new(conn),
                settings: RwLock::new(settings_val.clone()),
                session: session.clone(),
                images_dir,
                config_path,
                last_seen_hash: Mutex::new(None),
                suppress_until: Mutex::new(None),
                current_hotkey: Mutex::new(None),
                monitor_paused: AtomicBool::new(false),
                paste_backend: portal::new_cell(),
            });

            reconcile(&handle);
            let _ = tray::build(&handle);
            clipboard::start_monitor(handle.clone());

            // Install the panel hotkey using the session's detected backend
            // (GNOME custom keybinding, or the in-app global-shortcut plugin).
            commands::init_hotkey(&handle);

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_history,
            commands::search_history,
            commands::get_pins,
            commands::pin_item,
            commands::delete_item,
            commands::clear_history,
            commands::paste_item,
            commands::toggle_panel,
            commands::hide_panel,
            commands::get_settings,
            commands::set_settings,
            commands::set_hotkey,
            commands::get_session_info,
            commands::set_autostart,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

/// Startup reconcile: drop rows whose image file vanished, and delete orphan
/// files no longer referenced by the DB.
fn reconcile(app: &tauri::AppHandle) {
    let state = app.state::<AppState>();

    let rows = state.db.lock().map(|c| db::image_rows(&c)).unwrap_or_default();
    for (id, path) in rows {
        if let Some(p) = path {
            if !PathBuf::from(&p).exists() {
                if let Ok(conn) = state.db.lock() {
                    let _ = db::delete(&conn, id);
                }
            }
        }
    }

    let referenced: HashSet<PathBuf> = state
        .db
        .lock()
        .map(|c| db::referenced_image_paths(&c))
        .unwrap_or_default()
        .into_iter()
        .map(PathBuf::from)
        .collect();
    images::gc_orphans(&state.images_dir, &referenced);
}
