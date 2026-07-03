//! Lightweight "is there a newer release?" check against the GitHub Releases
//! API. This intentionally does *not* download or install anything: the app is
//! shipped as a `.deb` (which the Tauri updater can't manage) and an AppImage,
//! so we just detect a newer tag and point the user at the release page to grab
//! it. One blocking HTTPS GET (`ureq`), shared by the tray handler (off-thread)
//! and the `check_for_updates` command (via `spawn_blocking`).

use crate::models::UpdateCheck;
use std::time::Duration;

/// `owner/repo` for the GitHub Releases API + release page URLs.
const REPO: &str = "datluong2409/linux-clipboard";

/// The "latest release" page — used as the download target (and as a fallback
/// when the API response has no `html_url`).
pub fn releases_page() -> String {
    format!("https://github.com/{REPO}/releases/latest")
}

/// Blocking: ask GitHub for the latest release and compare its tag to `current`
/// (the running app version, e.g. "0.1.0"). Never panics — every failure maps
/// to an `error` code the UI can show.
pub fn check(current: &str) -> UpdateCheck {
    let url = format!("https://api.github.com/repos/{REPO}/releases/latest");
    let agent = ureq::AgentBuilder::new()
        .timeout(Duration::from_secs(10))
        .build();

    let resp = agent
        .get(&url)
        // GitHub rejects requests without a User-Agent; Accept pins the API version.
        .set("User-Agent", "linux-clipboard-update-check")
        .set("Accept", "application/vnd.github+json")
        .call();

    match resp {
        Ok(r) => match r.into_json::<serde_json::Value>() {
            Ok(json) => {
                let tag = json.get("tag_name").and_then(|v| v.as_str()).unwrap_or("");
                let latest = tag.trim_start_matches('v').trim().to_string();
                let page = json
                    .get("html_url")
                    .and_then(|v| v.as_str())
                    .map(str::to_string)
                    .unwrap_or_else(releases_page);
                UpdateCheck {
                    current_version: current.to_string(),
                    update_available: !latest.is_empty() && is_newer(&latest, current),
                    latest_version: (!latest.is_empty()).then_some(latest),
                    release_url: Some(page),
                    error: None,
                }
            }
            Err(_) => UpdateCheck::failed(current.to_string(), "parse"),
        },
        // 404 = no release published yet → nothing newer exists; treat as up-to-date.
        Err(ureq::Error::Status(404, _)) => UpdateCheck {
            current_version: current.to_string(),
            latest_version: None,
            update_available: false,
            release_url: None,
            error: None,
        },
        Err(_) => UpdateCheck::failed(current.to_string(), "network"),
    }
}

/// True if dotted-numeric `latest` is a higher version than `current`
/// ("0.2.0" > "0.1.9"). Non-numeric components parse as 0; differing lengths are
/// zero-padded, so "1.0" and "1.0.0" compare equal.
fn is_newer(latest: &str, current: &str) -> bool {
    fn parts(v: &str) -> Vec<u64> {
        v.split('.').map(|p| p.trim().parse().unwrap_or(0)).collect()
    }
    let (l, c) = (parts(latest), parts(current));
    for i in 0..l.len().max(c.len()) {
        let a = l.get(i).copied().unwrap_or(0);
        let b = c.get(i).copied().unwrap_or(0);
        if a != b {
            return a > b;
        }
    }
    false
}

/// Open `url` in the user's browser (Linux: `xdg-open`), fire-and-forget.
pub fn open_url(url: &str) {
    let _ = std::process::Command::new("xdg-open").arg(url).spawn();
}
