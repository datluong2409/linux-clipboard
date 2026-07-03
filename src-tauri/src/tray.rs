//! System tray icon + menu. On GNOME this requires the AppIndicator extension;
//! the menu (not click events) is the reliable interaction on Linux.
//!
//! The menu is rebuilt from `AppState` (via [`build_menu`]) so it can reflect
//! live status. The "Auto-paste" item is a toggle bound to
//! `Settings.auto_paste`; on Wayland, turning it on also runs the one-time
//! RemoteDesktop portal consent flow. Turning it off just stops pasting — the
//! OS grant stays cached so re-enabling is silent.

use crate::state::AppState;
use crate::window;
use tauri::menu::{Menu, MenuBuilder, MenuItemBuilder};
use tauri::tray::TrayIconBuilder;
use tauri::{AppHandle, Emitter, Manager, Wry};
use tauri_plugin_dialog::DialogExt;

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
            "toggle_paste" => on_toggle_paste(app),
            "quit" => app.exit(0),
            _ => {}
        });

    if let Some(icon) = app.default_window_icon() {
        builder = builder.icon(icon.clone());
    }

    builder.build(app)?;
    Ok(())
}

/// Rebuild the tray menu to reflect current state (auto-paste on/off, portal
/// grant). Must run on the main thread.
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

    // Auto-paste on/off toggle (only shown where auto-paste is possible at all).
    let state = app.state::<AppState>();
    if state.session.can_auto_paste {
        let label = match paste_state(&state) {
            PasteState::PortalMissing => "Auto-paste: thiếu portal ⚠",
            PasteState::NeedsPermission => "Auto-paste: cần cấp quyền ⚠",
            PasteState::On => "Auto-paste: Bật ✓",
            PasteState::Off => "Auto-paste: Tắt",
        };
        let toggle = MenuItemBuilder::with_id("toggle_paste", label).build(app)?;
        builder = builder.item(&toggle);
    }

    builder.item(&quit).build()
}

enum PasteState {
    /// Setting on and usable (granted, or X11 where no grant is needed).
    On,
    /// Setting off.
    Off,
    /// Setting on but the Wayland portal permission hasn't been granted yet.
    NeedsPermission,
    /// Setting on but no RemoteDesktop portal backend is installed
    /// (xdg-desktop-portal-gnome / -kde missing).
    PortalMissing,
}

fn paste_state(state: &AppState) -> PasteState {
    if !state.settings().auto_paste {
        return PasteState::Off;
    }
    if state.session.auto_paste_backend != "portal" {
        return PasteState::On; // X11 (enigo): nothing to grant
    }
    if !crate::portal::remote_desktop_available() {
        return PasteState::PortalMissing;
    }
    if !crate::portal::is_granted(&state.paste_backend) {
        PasteState::NeedsPermission
    } else {
        PasteState::On
    }
}

/// Handle a click on the auto-paste toggle, acting on the current state:
/// - `NeedsPermission` → run the consent flow, keep the setting on.
/// - `On` → turn the setting off (keeps the OS grant for later).
/// - `Off` → turn the setting on (+ grant now on Wayland).
fn on_toggle_paste(app: &AppHandle) {
    match paste_state(&app.state::<AppState>()) {
        PasteState::PortalMissing => warn_portal_missing(app),
        PasteState::NeedsPermission => grant_async(app),
        PasteState::On => {
            persist_auto_paste(app, false);
            refresh(app);
        }
        PasteState::Off => {
            persist_auto_paste(app, true);
            // Re-evaluate now that it's on and act on the resulting state:
            // grant now on Wayland (dialog), warn if the portal is missing, or
            // just refresh.
            match paste_state(&app.state::<AppState>()) {
                PasteState::PortalMissing => {
                    refresh(app);
                    warn_portal_missing(app);
                }
                PasteState::NeedsPermission => grant_async(app),
                _ => refresh(app),
            }
        }
    }
}

/// Warn (native dialog) that no RemoteDesktop portal backend is installed. Off
/// the main thread since `blocking_show` blocks.
fn warn_portal_missing(app: &AppHandle) {
    let app = app.clone();
    std::thread::spawn(move || {
        let _ = app
            .dialog()
            .message(crate::portal::PORTAL_MISSING_MSG)
            .title("Thiếu xdg-desktop-portal")
            .blocking_show();
    });
}

/// Write `Settings.auto_paste`, persist to disk, and notify the frontend —
/// mirroring `commands::set_settings` for this one field.
fn persist_auto_paste(app: &AppHandle, value: bool) {
    let state = app.state::<AppState>();
    let new_settings = {
        let Ok(mut g) = state.settings.write() else {
            return;
        };
        g.auto_paste = value;
        g.clone()
    };
    let _ = crate::settings::save(&state.config_path, &new_settings);
    let _ = app.emit("settings-updated", &new_settings);
}

/// Run the portal consent flow off the main thread (it blocks while the dialog
/// is up), then refresh the menu on the main thread.
fn grant_async(app: &AppHandle) {
    let app = app.clone();
    std::thread::spawn(move || {
        let cell = app.state::<AppState>().paste_backend.clone();
        if !crate::portal::enable(&cell) {
            eprintln!("[tray] auto-paste enabled but portal consent was denied/failed");
        }
        let app_ui = app.clone();
        let _ = app.run_on_main_thread(move || refresh(&app_ui));
    });
}
