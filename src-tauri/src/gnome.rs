//! Program (or remove) a GNOME custom keyboard shortcut via `gsettings`, so a
//! user-chosen key combo launches `<app> --toggle` — the Wayland-friendly path
//! to trigger the panel (single-instance forwards `--toggle` to the running app).

use std::process::Command;

const SLOT: &str =
    "/org/gnome/settings-daemon/plugins/media-keys/custom-keybindings/linux-clipboard/";
const SCHEMA: &str = "org.gnome.settings-daemon.plugins.media-keys";
const KB_SCHEMA: &str = "org.gnome.settings-daemon.plugins.media-keys.custom-keybinding";

pub fn configure(command: &str, gnome_accel: &str) -> Result<(), String> {
    let current = gsettings_get(&[SCHEMA, "custom-keybindings"]).unwrap_or_default();
    let list = ensure_slot(&current);
    gsettings_set(&[SCHEMA, "custom-keybindings", &list])?;

    let path_schema = format!("{KB_SCHEMA}:{SLOT}");
    gsettings_set(&[&path_schema, "name", "Linux Clipboard Toggle"])?;
    gsettings_set(&[&path_schema, "command", command])?;
    gsettings_set(&[&path_schema, "binding", gnome_accel])?;
    Ok(())
}

pub fn remove() -> Result<(), String> {
    let current = gsettings_get(&[SCHEMA, "custom-keybindings"]).unwrap_or_default();
    let list = remove_slot(&current);
    gsettings_set(&[SCHEMA, "custom-keybindings", &list])?;
    let path_schema = format!("{KB_SCHEMA}:{SLOT}");
    let _ = Command::new("gsettings")
        .args(["reset-recursively", &path_schema])
        .status();
    Ok(())
}

/// Translate a Tauri accelerator ("Ctrl+Alt+V") to a GNOME/GTK one ("<Control><Alt>v").
pub fn to_gnome_accel(tauri_accel: &str) -> String {
    let mut mods = String::new();
    let mut key = String::new();
    for part in tauri_accel.split('+') {
        match part.trim().to_lowercase().as_str() {
            "ctrl" | "control" | "commandorcontrol" => mods.push_str("<Control>"),
            "alt" | "option" => mods.push_str("<Alt>"),
            "shift" => mods.push_str("<Shift>"),
            "super" | "meta" | "cmd" | "command" | "win" => mods.push_str("<Super>"),
            k if !k.is_empty() => key = k.to_string(),
            _ => {}
        }
    }
    format!("{mods}{key}")
}

fn gsettings_get(args: &[&str]) -> Result<String, String> {
    let mut a = vec!["get"];
    a.extend_from_slice(args);
    let out = Command::new("gsettings")
        .args(&a)
        .output()
        .map_err(|e| e.to_string())?;
    if !out.status.success() {
        return Err(String::from_utf8_lossy(&out.stderr).trim().to_string());
    }
    Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

fn gsettings_set(args: &[&str]) -> Result<(), String> {
    let mut a = vec!["set"];
    a.extend_from_slice(args);
    let status = Command::new("gsettings")
        .args(&a)
        .status()
        .map_err(|e| e.to_string())?;
    if status.success() {
        Ok(())
    } else {
        Err("gsettings set failed".into())
    }
}

fn ensure_slot(current: &str) -> String {
    let mut items = parse_list(current);
    if !items.iter().any(|s| s == SLOT) {
        items.push(SLOT.to_string());
    }
    build_list(&items)
}

fn remove_slot(current: &str) -> String {
    let items: Vec<String> = parse_list(current)
        .into_iter()
        .filter(|s| s != SLOT)
        .collect();
    build_list(&items)
}

/// Parse a gsettings list value: handles "@as []", "[]", "['a', 'b']".
fn parse_list(s: &str) -> Vec<String> {
    let s = s.trim().trim_start_matches("@as").trim();
    let inner = s.trim_start_matches('[').trim_end_matches(']');
    inner
        .split(',')
        .map(|x| x.trim().trim_matches('\'').trim_matches('"').to_string())
        .filter(|x| !x.is_empty())
        .collect()
}

fn build_list(items: &[String]) -> String {
    if items.is_empty() {
        return "[]".to_string();
    }
    let joined = items
        .iter()
        .map(|i| format!("'{i}'"))
        .collect::<Vec<_>>()
        .join(", ");
    format!("[{joined}]")
}
