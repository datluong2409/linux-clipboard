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
use tauri_plugin_dialog::{DialogExt, MessageDialogButtons};

const TRAY_ID: &str = "main-tray";

pub fn build(app: &AppHandle) -> tauri::Result<()> {
    let menu = build_menu(app)?;

    let mut builder = TrayIconBuilder::with_id(TRAY_ID)
        .tooltip("Clipboard")
        .menu(&menu)
        .on_menu_event(|app, event| match event.id().as_ref() {
            "show" => window::show_panel(app),
            "settings" => {
                // Show first (this emits `panel-shown`, which resets the view
                // to the clipboard), *then* request settings — otherwise the
                // later `panel-shown` clobbers our navigation and we land on
                // the clipboard instead of settings.
                window::show_panel(app);
                let _ = app.emit("open-settings", ());
            }
            "toggle_paste" => on_toggle_paste(app),
            "check_updates" => on_check_updates(app),
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
    let state = app.state::<AppState>();
    let tr = state.lang();

    let show = MenuItemBuilder::with_id("show", tr.tray_show()).build(app)?;
    let settings = MenuItemBuilder::with_id("settings", tr.tray_settings()).build(app)?;
    let quit = MenuItemBuilder::with_id("quit", tr.tray_quit()).build(app)?;

    let mut builder = MenuBuilder::new(app).items(&[&show, &settings]);

    // Auto-paste on/off toggle (only shown where auto-paste is possible at all).
    if state.session.can_auto_paste {
        let label = match paste_state(&state) {
            PasteState::PortalMissing => tr.tray_auto_paste_portal_missing(),
            PasteState::NeedsPermission => tr.tray_auto_paste_needs_permission(),
            PasteState::On => tr.tray_auto_paste_on(),
            PasteState::Off => tr.tray_auto_paste_off(),
        };
        let toggle = MenuItemBuilder::with_id("toggle_paste", label).build(app)?;
        builder = builder.item(&toggle);
    }

    let check_updates =
        MenuItemBuilder::with_id("check_updates", tr.tray_check_updates()).build(app)?;
    builder = builder.item(&check_updates);

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

/// Handle a click on the tray auto-paste toggle, acting on the current state:
/// - `PortalMissing` → warn (nothing to toggle).
/// - `NeedsPermission` → run the consent flow, keep the setting on.
/// - `On` → turn the setting off (keeps the OS grant for later).
/// - `Off` → turn the setting on (+ grant now on Wayland).
fn on_toggle_paste(app: &AppHandle) {
    match paste_state(&app.state::<AppState>()) {
        PasteState::PortalMissing => warn_portal_missing(app),
        PasteState::NeedsPermission => grant_async(app),
        PasteState::On => apply_auto_paste(app, false),
        PasteState::Off => apply_auto_paste(app, true),
    }
}

/// Check GitHub for a newer release, off the main thread (the network GET and
/// the native dialogs both block). On success either offer to open the release
/// page (newer version) or confirm the app is up to date; on failure, warn.
fn on_check_updates(app: &AppHandle) {
    let app = app.clone();
    std::thread::spawn(move || {
        let current = app.package_info().version.to_string();
        let result = crate::updater::check(&current);
        let tr = app.state::<AppState>().lang();

        if result.error.is_some() {
            let _ = app
                .dialog()
                .message(format!("{}\n\n{}", tr.update_error_title(), tr.update_error_body()))
                .title(tr.app_title())
                .blocking_show();
            return;
        }

        if result.update_available {
            let latest = result.latest_version.as_deref().unwrap_or("");
            let open = app
                .dialog()
                .message(format!(
                    "{}\n\n{}",
                    tr.update_available_title(),
                    tr.update_available_body(latest, &result.current_version)
                ))
                .title(tr.app_title())
                .buttons(MessageDialogButtons::OkCancelCustom(
                    tr.update_open().into(),
                    tr.update_later().into(),
                ))
                .blocking_show();
            if open {
                let url = result
                    .release_url
                    .unwrap_or_else(crate::updater::releases_page);
                crate::updater::open_url(&url);
            }
        } else {
            let _ = app
                .dialog()
                .message(format!(
                    "{}\n\n{}",
                    tr.update_up_to_date_title(),
                    tr.update_up_to_date_body(&result.current_version)
                ))
                .title(tr.app_title())
                .blocking_show();
        }
    });
}

/// Set `Settings.auto_paste` and run the same follow-up as the tray toggle so
/// the tray and the Settings UI behave identically: enabling on Wayland grants
/// the RemoteDesktop permission now (or warns if no portal backend exists);
/// disabling just stops (the OS grant stays cached). Shared with
/// `commands::set_auto_paste`.
pub(crate) fn apply_auto_paste(app: &AppHandle, enabled: bool) {
    persist_auto_paste(app, enabled);
    if !enabled {
        refresh(app);
        return;
    }
    // Re-evaluate now that it's on and act on the resulting state.
    match paste_state(&app.state::<AppState>()) {
        PasteState::PortalMissing => {
            refresh(app);
            warn_portal_missing(app);
        }
        PasteState::NeedsPermission => grant_async(app),
        _ => refresh(app),
    }
}

/// Current auto-paste state as a stable string for the Settings UI:
/// "on" | "off" | "needs_permission" | "portal_missing".
pub(crate) fn auto_paste_status(state: &AppState) -> &'static str {
    match paste_state(state) {
        PasteState::On => "on",
        PasteState::Off => "off",
        PasteState::NeedsPermission => "needs_permission",
        PasteState::PortalMissing => "portal_missing",
    }
}

/// Warn (native dialog) that no RemoteDesktop portal backend is installed. Off
/// the main thread since `blocking_show` blocks.
fn warn_portal_missing(app: &AppHandle) {
    let app = app.clone();
    std::thread::spawn(move || {
        let tr = app.state::<AppState>().lang();
        let _ = app
            .dialog()
            .message(format!("{}\n\n{}", tr.portal_missing_title(), tr.portal_missing_body()))
            .title(tr.app_title())
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
        let _ = app.run_on_main_thread(move || {
            refresh(&app_ui);
            // The grant flips the paste state (needs_permission → on) without
            // changing Settings; reuse `settings-updated` to nudge the Settings
            // UI to re-read it.
            let settings = app_ui.state::<AppState>().settings();
            let _ = app_ui.emit("settings-updated", &settings);
        });
    });
}
