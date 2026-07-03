# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Finding code

This repo has a `.codegraph/` index. Use CodeGraph (the `codegraph_explore` MCP tool, or `codegraph explore "<symbols or question>"` in the shell) instead of grep/find/Read when locating or understanding code — it returns verbatim symbol source plus call paths in one call.

## What this is

A Windows-10-style clipboard history manager for Linux (Ubuntu/GNOME): **Tauri v2 + React 19 + TypeScript + Tailwind v4** frontend, **Rust** backend. Background service that records clipboard text/images and pops up a panel on a hotkey to pick and paste an item.

## Commands

```bash
npm install            # install frontend deps
npm run dev:app         # run the app in dev mode (hot-reload frontend); sets software-GL env vars for compatibility
npm run build:app       # produce .deb / AppImage via `fakeroot tauri build` (src-tauri/target/release/bundle)
npm run dev             # vite only (frontend, no Tauri shell)
npm run build           # tsc + vite build (frontend only)
```

There is no configured lint or test suite (no eslint config, no test runner/files in `src` or `src-tauri`). Type-check the frontend with `tsc` (via `npm run build`); check the backend with `cargo check` / `cargo build` from `src-tauri/`.

System build deps (Ubuntu) are listed in [README.md](README.md) — WebKitGTK, `libxdo-dev` (enigo/X11), `libxkbcommon-dev` (Wayland portal paste), `libayatana-appindicator3-dev` (tray), `patchelf` (AppImage). Wayland auto-paste uses the XDG RemoteDesktop portal (`xdg-desktop-portal-gnome`/`-kde`, shipped by default) — no ydotool/uinput.

## Architecture

**The Rust backend owns all state; the React frontend is a thin, mostly-stateless view.** Every mutation (clipboard read/write, DB, settings, hotkey registration) happens in Rust behind `#[tauri::command]`s in `src-tauri/src/commands.rs`; the frontend calls them via typed wrappers in [src/lib/ipc.ts](src/lib/ipc.ts) and reacts to two backend-emitted events (`history-updated`, `settings-updated`, plus `panel-shown` / `open-settings` for navigation) via `onEvent`.

Shared types (`Clip`, `Settings`, `SessionInfo`, `OpResult`) are defined once in `src-tauri/src/models.rs` (serde, camelCase over IPC) and mirrored by hand in [src/types.ts](src/types.ts) — keep both in sync when changing a field.

### Backend module map (`src-tauri/src/`)

- `lib.rs` — app entrypoint: registers plugins (single-instance, global-shortcut, autostart), builds `AppState`, wires the `invoke_handler` command list, runs startup `reconcile()` (drops DB rows for missing image files, GCs orphan image files).
- `state.rs` — `AppState`, the single managed struct holding the DB connection, in-memory settings, session info, and the clipboard-echo suppression window (`arm_suppress`/`consume_suppress`) that stops paste-back writes from being re-recorded as new history.
- `commands.rs` — the entire frontend-callable surface; thin glue that locks `AppState.db`, delegates to `db`/`clipboard`/`images`/`hotkey`/`gnome`/`autostart`/`window`, and emits update events.
- `clipboard.rs` — the polling monitor (400ms, single background thread owning one `arboard::Clipboard`) plus paste-back writers. Dedup: identical content already in history bumps `last_used_at` instead of inserting. Images take priority over text when both are present.
- `db.rs` — `rusqlite` (bundled SQLite) queries: history/pins listing, search, insert, hash lookup (dedup), history-cap enforcement (returns files to GC), delete/clear.
- `images.rs` — saves clipboard images as PNG + thumbnail on disk, deletes files for GC'd rows, cleans orphans on startup.
- `session.rs` — detects X11 vs Wayland (`XDG_SESSION_TYPE`/env fallback) and derives capability flags (`can_global_shortcut`, `can_auto_paste`, `auto_paste_backend`) consumed everywhere else to degrade gracefully.
- `paste.rs` — synthetic Ctrl+V dispatch: `enigo` on X11, the `portal` backend on Wayland; also X11 cursor location for panel placement. `paste()` takes the `SessionInfo` and the `PortalCell`.
- `portal.rs` — Wayland auto-paste via the XDG RemoteDesktop portal + libei (`ashpd`/`reis`/`xkbcommon`), extracted/trimmed from the sibling `wdotool` project. Keyboard-only; sends Ctrl+V *through the compositor* (no `/dev/uinput`). Lazily builds one persistent portal session (held in `AppState.paste_backend`, a `PortalCell`) on the first paste — triggering a one-time consent dialog — and caches the `restore_token` at `$XDG_STATE_HOME/linux-clipboard/portal.token` so later sessions are silent. Degrades to copy-only if the portal is unavailable.
- `hotkey.rs` — registers/rebinds the in-app global shortcut (X11 only).
- `gnome.rs` — configures a GNOME custom keybinding (via `gsettings`) that runs `<app-binary> --toggle`, the Wayland-compatible trigger path.
- `window.rs` — the single show/hide/toggle path for the panel window; cursor-relative positioning on X11, centered on Wayland.
- `tray.rs` — tray icon + menu, rebuilt from `AppState` via `build_menu`/`refresh`. On Wayland it shows the auto-paste (portal) status and an "enable" item that runs the one-time consent flow off-thread, then refreshes the label.
- `autostart.rs`, `settings.rs` (JSON persistence), `util.rs` (content hashing for dedup/suppress).

### Frontend (`src/`)

- `App.tsx` — switches between the `panel` and `settings` views; owns window focus-loss auto-hide (mimicking Win+V), driven by the backend's `panel-shown`/`open-settings` events.
- `hooks/useHistory.ts`, `hooks/useSettings.ts` — data-fetching hooks wrapping the `ipc.ts` calls.
- `components/ClipboardPanel.tsx` — the main list view (history + pins + search).
- `views/SettingsView.tsx` — hotkey rebinding, GNOME shortcut helper, auto-paste/theme/history-cap settings; reads `SessionInfo` to show the right UI for X11 vs Wayland.
- `lib/accelerator.ts` — accelerator string parsing/formatting for hotkey capture UI.
- Images are referenced by absolute filesystem path from the DB and turned into loadable `asset:` URLs via `assetUrl()` (`convertFileSrc`), permitted by the `assetProtocol` scope in `src-tauri/tauri.conf.json`.

### X11 vs Wayland — the central constraint

Nearly every backend module branches on `SessionInfo` because Wayland disallows global input hotkeys and reliable synthetic input:

- Hotkey trigger: in-app global shortcut (X11) vs GNOME custom keybinding running `<app> --toggle`, forwarded to the running instance via `tauri-plugin-single-instance` (works on both).
- Auto-paste: `enigo` (X11) vs the RemoteDesktop portal + libei (`portal.rs`, Wayland) vs copy-only fallback.
- Panel positioning: cursor-relative (X11) vs centered (Wayland).

When touching hotkey/paste/positioning code, check `session.rs` capability flags rather than assuming a display server.

### Data locations (runtime, not in repo)

- History DB: `~/.local/share/com.datluong.linuxclipboard/history.db`
- Images: `~/.local/share/com.datluong.linuxclipboard/images/`
- Settings: `~/.config/com.datluong.linuxclipboard/settings.json`
