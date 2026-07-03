//! System tray icon + menu. On GNOME this requires the AppIndicator extension;
//! the menu (not click events) is the reliable interaction on Linux.
//!
//! The menu is rebuilt from `AppState` (via [`build_menu`]) so it can reflect
//! live status — notably the Wayland auto-paste permission, which the user
//! grants once through the RemoteDesktop portal consent dialog.

use crate::state::AppState;
use crate::window;
use tauri::menu::{Menu, MenuBuilder, MenuItemBuilder};
use tauri::tray::TrayIconBuilder;
use tauri::{AppHandle, Emitter, Manager, Wry};

const TRAY_ID: &str = "main-tray";

pub fn build(app: &AppHandle) -> tauri::Result<()> {
    let menu = build_menu(app)?;

    let mut builder = TrayIconBuilder::with_id(TRAY_ID)
        .tooltip("Clipboard")
        .menu(&menu)
        .on_menu_event(|app, event| match event.id().as_ref() {
            "show" => window::show_panel(app),
            "settings" => {
                let _ = app.emit("open-settings", ());
                window::show_panel(app);
            }
            "enable_paste" => on_enable_paste(app),
            "quit" => app.exit(0),
            _ => {}
        });

    if let Some(icon) = app.default_window_icon() {
        builder = builder.icon(icon.clone());
    }

    builder.build(app)?;
    Ok(())
}

/// Rebuild the tray menu to reflect current state (e.g. after auto-paste is
/// enabled). Must run on the main thread.
pub fn refresh(app: &AppHandle) {
    if let Some(tray) = app.tray_by_id(TRAY_ID) {
        if let Ok(menu) = build_menu(app) {
            let _ = tray.set_menu(Some(menu));
        }
    }
}

fn build_menu(app: &AppHandle) -> tauri::Result<Menu<Wry>> {
    let show = MenuItemBuilder::with_id("show", "Show clipboard").build(app)?;
    let settings = MenuItemBuilder::with_id("settings", "Settings").build(app)?;
    let quit = MenuItemBuilder::with_id("quit", "Quit").build(app)?;

    let mut builder = MenuBuilder::new(app).items(&[&show, &settings]);

    // On Wayland the auto-paste backend is the RemoteDesktop portal, which needs
    // a one-time consent. Surface its status + an enable action right here so
    // the user can grant it when convenient instead of mid-paste.
    let state = app.state::<AppState>();
    if state.session.auto_paste_backend == "portal" {
        let label = if crate::portal::is_granted(&state.paste_backend) {
            "Auto-paste: Đã bật ✓"
        } else {
            "Bật auto-paste (cấp quyền)…"
        };
        let enable = MenuItemBuilder::with_id("enable_paste", label).build(app)?;
        builder = builder.item(&enable);
    }

    builder.item(&quit).build()
}

/// Run the portal consent flow off the main thread (it blocks while the dialog
/// is up), then rebuild the menu on the main thread to update the status label.
fn on_enable_paste(app: &AppHandle) {
    let app = app.clone();
    std::thread::spawn(move || {
        let cell = app.state::<AppState>().paste_backend.clone();
        if !crate::portal::enable(&cell) {
            eprintln!("[tray] enabling auto-paste failed or was denied");
        }
        let app_ui = app.clone();
        let _ = app.run_on_main_thread(move || refresh(&app_ui));
    });
}
