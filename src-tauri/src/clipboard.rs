//! Clipboard access + the polling monitor.
//!
//! A single background thread owns one `arboard::Clipboard` and polls it. On a
//! detected change it applies suppression (ignore our own writes) and dedup
//! (re-copying existing content moves it to the top), then records to the DB.
//!
//! Writes for paste-back run on their own short-lived threads that hold the
//! X11 selection alive (`set().wait()`) until another app takes ownership —
//! this is required on X11, where the clipboard is ownership-based, not a store.

use crate::models::Settings;
use crate::state::AppState;
use crate::util::{hash_bytes, hash_text};
use crate::{db, images};
use arboard::Clipboard;
use std::sync::atomic::Ordering;
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager};

const POLL_MS: u64 = 400;

pub enum Payload {
    Text(String),
    Image {
        width: usize,
        height: usize,
        rgba: Vec<u8>,
        hash: String,
    },
    None,
}

/// Read the current clipboard, preferring images (the richer type) over text.
fn read_with(cb: &mut Clipboard) -> Payload {
    if let Ok(img) = cb.get_image() {
        let (width, height) = (img.width, img.height);
        let hash = hash_bytes(&img.bytes);
        return Payload::Image {
            width,
            height,
            rgba: img.bytes.into_owned(),
            hash,
        };
    }
    if let Ok(text) = cb.get_text() {
        if !text.is_empty() {
            return Payload::Text(text);
        }
    }
    Payload::None
}

/// Spawn the monitor thread. Owns one Clipboard for the app's lifetime.
pub fn start_monitor(app: AppHandle) {
    std::thread::spawn(move || {
        let mut clipboard = Clipboard::new().ok();
        loop {
            std::thread::sleep(Duration::from_millis(POLL_MS));
            let state = app.state::<AppState>();
            let st: &AppState = &state;
            if st.monitor_paused.load(Ordering::Relaxed) {
                continue;
            }
            if clipboard.is_none() {
                clipboard = Clipboard::new().ok();
            }
            let Some(cb) = clipboard.as_mut() else {
                continue;
            };

            let payload = read_with(cb);
            let hash = match &payload {
                Payload::Text(t) => hash_text(t),
                Payload::Image { hash, .. } => hash.clone(),
                Payload::None => continue,
            };

            // No change since last poll.
            if st.last_seen().as_deref() == Some(hash.as_str()) {
                continue;
            }
            st.set_last_seen(Some(hash.clone()));

            // Our own paste-back write echoing back — skip it.
            if st.consume_suppress() {
                continue;
            }

            let settings = st.settings();
            if record(st, &settings, payload, &hash) {
                let _ = app.emit("history-updated", ());
            }
        }
    });
}

/// Persist a clipboard payload. Returns true if the UI should refresh.
fn record(st: &AppState, settings: &Settings, payload: Payload, hash: &str) -> bool {
    let Ok(conn) = st.db.lock() else {
        return false;
    };

    // Dedup: identical content already in history -> move it to the top.
    if let Some(id) = db::find_by_hash(&conn, hash) {
        db::bump_used(&conn, id);
        return true;
    }

    let gc = match payload {
        Payload::Text(text) => {
            let new = db::NewClip {
                kind: "text",
                content: Some(&text),
                image_path: None,
                thumb_path: None,
                width: None,
                height: None,
                byte_size: Some(text.len() as i64),
                hash,
            };
            if db::insert(&conn, &new).is_err() {
                return false;
            }
            db::enforce_cap(&conn, settings.history_cap)
        }
        Payload::Image {
            width,
            height,
            rgba,
            ..
        } => {
            if !settings.capture_images {
                return false;
            }
            let Some(saved) = images::save(&st.images_dir, hash, width, height, &rgba) else {
                return false;
            };
            // Drop oversized images once we know the encoded size.
            if saved.byte_size as u64 > settings.max_image_bytes {
                images::delete_files(&[(
                    Some(saved.image_path.clone()),
                    Some(saved.thumb_path.clone()),
                )]);
                return false;
            }
            let new = db::NewClip {
                kind: "image",
                content: None,
                image_path: Some(&saved.image_path),
                thumb_path: Some(&saved.thumb_path),
                width: Some(saved.width),
                height: Some(saved.height),
                byte_size: Some(saved.byte_size),
                hash,
            };
            if db::insert(&conn, &new).is_err() {
                return false;
            }
            db::enforce_cap(&conn, settings.history_cap)
        }
        Payload::None => return false,
    };

    drop(conn);
    images::delete_files(&gc);
    true
}

/// Set clipboard text (armed against self-recording). Keeps the X11 selection
/// alive on a background thread until another app takes ownership.
pub fn write_text(st: &AppState, text: String) {
    st.arm_suppress();
    st.set_last_seen(Some(hash_text(&text)));
    std::thread::spawn(move || {
        if let Ok(mut cb) = Clipboard::new() {
            use arboard::SetExtLinux;
            let _ = cb.set().wait().text(text);
        }
    });
}

/// Load a stored PNG and place it back on the clipboard.
pub fn write_image_from_path(st: &AppState, path: &str) -> bool {
    let Ok(dynimg) = image::open(path) else {
        return false;
    };
    let rgba = dynimg.to_rgba8();
    let (width, height) = (rgba.width() as usize, rgba.height() as usize);
    let bytes = rgba.into_raw();
    st.arm_suppress();
    st.set_last_seen(Some(hash_bytes(&bytes)));
    std::thread::spawn(move || {
        if let Ok(mut cb) = Clipboard::new() {
            use arboard::SetExtLinux;
            let img = arboard::ImageData {
                width,
                height,
                bytes: std::borrow::Cow::Owned(bytes),
            };
            let _ = cb.set().wait().image(img);
        }
    });
    true
}
