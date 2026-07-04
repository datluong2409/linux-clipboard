// Typed wrappers around the Rust command surface + event helpers.
// Tauri maps camelCase JS keys to snake_case Rust params automatically.

import { invoke, convertFileSrc } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type {
  Clip,
  OpResult,
  PasteState,
  SessionInfo,
  Settings,
  UpdateCheck,
} from "../types";

export const getHistory = (limit?: number) =>
  invoke<Clip[]>("get_history", { limit });

export const searchHistory = (query: string, limit?: number) =>
  invoke<Clip[]>("search_history", { query, limit });

export const getPins = () => invoke<Clip[]>("get_pins");

export const pinItem = (id: number, pinned: boolean) =>
  invoke("pin_item", { id, pinned });

export const deleteItem = (id: number) => invoke("delete_item", { id });

export const clearHistory = (keepPinned: boolean) =>
  invoke("clear_history", { keepPinned });

export const pasteItem = (id: number) => invoke<OpResult>("paste_item", { id });

export const hidePanel = () => invoke("hide_panel");

export const togglePanel = () => invoke("toggle_panel");

export const getSettings = () => invoke<Settings>("get_settings");

export const saveSettings = (settings: Settings) =>
  invoke<OpResult>("set_settings", { settings });

export const setHotkey = (accel: string) =>
  invoke<OpResult>("set_hotkey", { accel });

/** The exact `<app> --toggle` command to bind manually where there's no automatic hotkey backend. */
export const getToggleCommand = () => invoke<string>("get_toggle_command");

export const getSessionInfo = () => invoke<SessionInfo>("get_session_info");

/** Turn auto-paste on/off, running the same grant/warn logic as the tray. */
export const setAutoPaste = (enabled: boolean) =>
  invoke("set_auto_paste", { enabled });

export const getPasteState = () => invoke<PasteState>("get_paste_state");

/** The running app version (e.g. "0.1.0"). */
export const getVersion = () => invoke<string>("get_version");

/** Ask the backend to check GitHub Releases for a newer version. */
export const checkForUpdates = () => invoke<UpdateCheck>("check_for_updates");

/** Open a release URL (or the latest-release page when omitted) in the browser. */
export const openReleasePage = (url?: string | null) =>
  invoke("open_release_page", { url: url ?? null });

/** Turn an absolute file path into an asset: URL the webview can load. */
export const assetUrl = (path: string) => convertFileSrc(path);

/** Subscribe to a backend event; returns the unlisten promise. */
export const onEvent = (event: string, cb: () => void): Promise<UnlistenFn> =>
  listen(event, () => cb());
