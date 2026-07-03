// Mirror of the Rust serde types (camelCase over IPC).

export interface Clip {
  id: number;
  kind: "text" | "image";
  content: string | null;
  imagePath: string | null;
  thumbPath: string | null;
  width: number | null;
  height: number | null;
  byteSize: number | null;
  pinned: boolean;
  createdAt: number;
  lastUsedAt: number;
}

export interface Settings {
  hotkey: string;
  autoPaste: boolean;
  autostart: boolean;
  historyCap: number;
  captureImages: boolean;
  maxImageBytes: number;
  hideOnBlur: boolean;
  positionMode: "cursor" | "center";
  theme: "system" | "light" | "dark";
  gnomeShortcutConfigured: boolean;
  firstRunDone: boolean;
}

export interface SessionInfo {
  kind: "x11" | "wayland" | "unknown";
  isGnome: boolean;
  canGlobalShortcut: boolean;
  /** Which mechanism the panel hotkey uses in this session. */
  hotkeyBackend: "gnome" | "global-shortcut" | "none";
  canAutoPaste: boolean;
  autoPasteBackend: "enigo" | "portal" | "none";
}

export interface OpResult {
  ok: boolean;
  reason?: string | null;
}
