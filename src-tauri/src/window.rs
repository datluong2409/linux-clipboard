//! The single unified show/hide/toggle path for the panel window, plus
//! cursor-relative positioning (X11) with a centered fallback (Wayland).

use crate::state::AppState;
use tauri::{AppHandle, Emitter, LogicalSize, Manager, PhysicalPosition, WebviewWindow};

pub fn get_panel(app: &AppHandle) -> Option<WebviewWindow> {
    app.get_webview_window("main")
}

/// Toggle visibility. Every trigger (hotkey, tray, `--toggle`) funnels here.
pub fn toggle(app: &AppHandle) {
    if let Some(win) = get_panel(app) {
        if win.is_visible().unwrap_or(false) {
            let _ = win.hide();
        } else {
            show(app, &win);
        }
    }
}

pub fn show_panel(app: &AppHandle) {
    if let Some(win) = get_panel(app) {
        show(app, &win);
    }
}

pub fn hide_panel(app: &AppHandle) {
    if let Some(win) = get_panel(app) {
        let _ = win.hide();
    }
}

fn show(app: &AppHandle, win: &WebviewWindow) {
    position(app, win);
    let _ = win.show();
    let _ = win.set_always_on_top(true);
    let _ = win.set_focus();
    // Front-end loads history + focuses search on this event.
    let _ = app.emit("panel-shown", ());

    // X11/GNOME (Mutter's focus-stealing prevention) frequently drops the focus
    // request issued the instant the window maps, leaving the panel visible but
    // without keyboard focus. Re-assert focus once the window has had time to
    // map so search/arrow-keys work immediately on show. Guarded on still-being
    // -visible so a quick toggle-off within the delay doesn't resurrect it.
    let win = win.clone();
    std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_millis(80));
        if win.is_visible().unwrap_or(false) {
            let _ = win.set_focus();
        }
    });
}

fn position(app: &AppHandle, win: &WebviewWindow) {
    let state = app.state::<AppState>();
    let settings = state.settings();

    // Panel size, same for every session.
    let _ = win.set_size(LogicalSize::new(380.0, 520.0));

    // Wayland (or forced center): let the compositor place it.
    if settings.position_mode != "cursor" || state.session.kind != "x11" {
        let _ = win.center();
        return;
    }

    let Some((cx, cy)) = crate::paste::cursor_location() else {
        let _ = win.center();
        return;
    };

    let (w, h) = win
        .outer_size()
        .map(|s| (s.width as i32, s.height as i32))
        .unwrap_or((460, 520));

    let mut x = cx + 8;
    let mut y = cy + 8;

    // Clamp within the monitor *under the cursor* so the panel never spills off
    // — and, on a multi-monitor layout, so it actually lands on the screen the
    // cursor is on. `current_monitor()` returns the monitor the *window* sits on
    // (usually the primary), which would wrongly clamp a cursor on a second
    // monitor back onto the first; find the monitor that contains the cursor.
    let monitor = win
        .available_monitors()
        .ok()
        .and_then(|mons| {
            mons.into_iter().find(|m| {
                let p = m.position();
                let s = m.size();
                cx >= p.x
                    && cx < p.x + s.width as i32
                    && cy >= p.y
                    && cy < p.y + s.height as i32
            })
        })
        .or_else(|| win.current_monitor().ok().flatten());

    if let Some(monitor) = monitor {
        let mp = monitor.position();
        let ms = monitor.size();
        let (min_x, min_y) = (mp.x, mp.y);
        let (max_x, max_y) = (
            mp.x + ms.width as i32 - w,
            mp.y + ms.height as i32 - h,
        );
        if x > max_x {
            x = (cx - w - 8).max(min_x);
        }
        if y > max_y {
            y = (cy - h - 8).max(min_y);
        }
        x = x.clamp(min_x, max_x.max(min_x));
        y = y.clamp(min_y, max_y.max(min_y));
    }

    let _ = win.set_position(PhysicalPosition::new(x, y));
}
