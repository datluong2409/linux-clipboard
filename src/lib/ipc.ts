// Typed wrappers around the Rust command surface + event helpers.
// Tauri maps camelCase JS keys to snake_case Rust params automatically.

import { invoke, convertFileSrc } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type { Clip, OpResult, SessionInfo, Settings } from "../types";

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

export const getSessionInfo = () => invoke<SessionInfo>("get_session_info");

export const configureGnomeShortcut = (accel: string) =>
  invoke<OpResult>("configure_gnome_shortcut", { accel });

export const removeGnomeShortcut = () =>
  invoke<OpResult>("remove_gnome_shortcut");

export const setAutostart = (enabled: boolean) =>
  invoke<OpResult>("set_autostart", { enabled });

/** Turn an absolute file path into an asset: URL the webview can load. */
export const assetUrl = (path: string) => convertFileSrc(path);

/** Subscribe to a backend event; returns the unlisten promise. */
export const onEvent = (event: string, cb: () => void): Promise<UnlistenFn> =>
  listen(event, () => cb());
