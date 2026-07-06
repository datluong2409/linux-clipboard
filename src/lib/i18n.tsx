// Lightweight i18n for the frontend. English is the source of truth (its keys
// define the type); every other language must provide the same keys or this
// file fails to type-check. The active language comes from `Settings.language`,
// which the Rust backend owns — see `App.tsx` for where the provider is wired.

import { createContext, useContext, useEffect, useMemo, type ReactNode } from "react";

export type Lang = "en" | "vi";

const en = {
  // Clipboard panel
  searchPlaceholder: "Search…",
  pinned: "Pinned",
  recent: "Recent",
  clearHistoryKeepPinned: "Clear history (keep pinned)",
  settings: "Settings",
  copiedPressCtrlV: "Copied — press Ctrl+V to paste",

  // Clipboard item actions
  pin: "Pin",
  unpin: "Unpin",
  delete: "Delete",
  formatted: "Formatted text",
  pastePlain: "Paste as plain text",

  // Empty state
  emptyNoResults: "No results found",
  emptyNothing: "Nothing in clipboard history yet",
  emptyHint: "Copy (Ctrl+C) anywhere to get started.",

  // Settings — chrome
  back: "Back",
  loading: "Loading…",

  // Settings — hotkey
  panelHotkey: "Open-panel hotkey",
  pressCombo: "Press a key combo… (Esc to cancel)",
  hotkeyGnomeSync:
    "Automatically synced with the GNOME system shortcut (applies immediately, even on Wayland).",
  hotkeyNoneExplain:
    "This environment (Wayland outside GNOME) can't let the app register a global hotkey. The hotkey is still saved, but to open the panel from the keyboard, bind this command to a shortcut in your desktop's keyboard settings:",
  hotkeyNoneHowto:
    "For example — KDE: System Settings › Shortcuts (add a custom shortcut). Hyprland / Sway: add a bind line to your config.",
  copy: "Copy",
  copied: "Command copied",
  copyFailed: "Couldn't copy — select and copy it manually",

  // Settings — auto-paste
  autoPasteLabel: "Auto-paste",
  autoPasteHint: "Paste straight into the active app when you pick an item",
  needsPermission:
    "Remote Desktop permission (one-time) is required to auto-paste on Wayland.",
  grantPermission: "Grant permission",
  portalMissing:
    "No xdg-desktop-portal backend (gnome/kde), so auto-paste isn't possible. The content is still copied for you to paste with Ctrl+V.",

  // Settings — updates
  updates: "Updates",
  currentVersion: "Current version",
  checkForUpdates: "Check for updates",
  checkingUpdates: "Checking…",
  upToDate: "You're on the latest version.",
  updateAvailable: "New version available:",
  downloadUpdate: "Download",
  updateCheckFailed: "Couldn't check for updates. Check your connection and try again.",

  // Settings — preferences
  language: "Language",
  maxItems: "Max items",
  theme: "Theme",
  themeSystem: "System",
  themeLight: "Light",
  themeDark: "Dark",

  // Settings — author (shown in the Updates section)
  author: "Author",

  // Settings — session info
  displaySession: "Display session",
  hotkeyMechanism: "Hotkey mechanism",
  autoPasteBackend: "Auto-paste backend",

  // Toasts / hotkey validation
  hotkeyChanged: "Hotkey changed",
  hotkeySavedManual: "Hotkey saved — this environment needs a manual shortcut",
  invalidCombo: "Invalid combination",
  registerFailed: "Couldn't register (may already be in use)",
  needMainKey: "Need a main key, not just modifiers.",
  needModifier: "Need at least one modifier (Ctrl / Alt / Super).",
} as const;

export type TKey = keyof typeof en;

const vi: Record<TKey, string> = {
  searchPlaceholder: "Tìm kiếm…",
  pinned: "Đã ghim",
  recent: "Gần đây",
  clearHistoryKeepPinned: "Xoá lịch sử (giữ ghim)",
  settings: "Cài đặt",
  copiedPressCtrlV: "Đã copy — nhấn Ctrl+V để dán",

  pin: "Ghim",
  unpin: "Bỏ ghim",
  delete: "Xoá",
  formatted: "Văn bản có định dạng",
  pastePlain: "Dán dạng text thuần",

  emptyNoResults: "Không tìm thấy kết quả",
  emptyNothing: "Chưa có gì trong lịch sử clipboard",
  emptyHint: "Copy (Ctrl+C) ở bất kỳ đâu để bắt đầu.",

  back: "Quay lại",
  loading: "Đang tải…",

  panelHotkey: "Phím tắt mở bảng",
  pressCombo: "Nhấn tổ hợp phím… (Esc để huỷ)",
  hotkeyGnomeSync:
    "Tự động đồng bộ với shortcut hệ thống GNOME (áp dụng ngay, kể cả trên Wayland).",
  hotkeyNoneExplain:
    "Môi trường này (Wayland ngoài GNOME) không cho app tự đăng ký phím tắt. Phím tắt vẫn được lưu, nhưng để mở bảng bằng bàn phím, hãy gán lệnh sau vào một shortcut trong cài đặt bàn phím của desktop:",
  hotkeyNoneHowto:
    "Ví dụ — KDE: System Settings › Shortcuts (thêm shortcut tuỳ chỉnh). Hyprland / Sway: thêm dòng bind vào file cấu hình.",
  copy: "Sao chép",
  copied: "Đã sao chép lệnh",
  copyFailed: "Không sao chép được — hãy tự bôi đen và copy",

  autoPasteLabel: "Tự động dán (auto-paste)",
  autoPasteHint: "Dán thẳng vào app đang mở khi chọn 1 mục",
  needsPermission:
    "Cần cấp quyền Remote Desktop (một lần) để tự động dán trên Wayland.",
  grantPermission: "Cấp quyền",
  portalMissing:
    "Chưa có backend xdg-desktop-portal (gnome/kde) nên không thể tự động dán. Nội dung vẫn được copy để bạn tự dán bằng Ctrl+V.",

  updates: "Cập nhật",
  currentVersion: "Phiên bản hiện tại",
  checkForUpdates: "Kiểm tra cập nhật",
  checkingUpdates: "Đang kiểm tra…",
  upToDate: "Bạn đang dùng phiên bản mới nhất.",
  updateAvailable: "Đã có phiên bản mới:",
  downloadUpdate: "Tải về",
  updateCheckFailed: "Không thể kiểm tra cập nhật. Hãy kiểm tra kết nối mạng và thử lại.",

  language: "Ngôn ngữ",
  maxItems: "Số mục tối đa",
  theme: "Giao diện",
  themeSystem: "Hệ thống",
  themeLight: "Sáng",
  themeDark: "Tối",

  author: "Tác giả",

  displaySession: "Phiên hiển thị",
  hotkeyMechanism: "Cơ chế phím tắt",
  autoPasteBackend: "Cơ chế auto-paste",

  hotkeyChanged: "Đã đổi phím tắt",
  hotkeySavedManual:
    "Đã lưu phím tắt — môi trường này cần tạo shortcut thủ công",
  invalidCombo: "Tổ hợp không hợp lệ",
  registerFailed: "Không đăng ký được (có thể đã bị dùng)",
  needMainKey: "Cần một phím chính, không chỉ phím bổ trợ.",
  needModifier: "Cần ít nhất một phím bổ trợ (Ctrl / Alt / Super).",
};

const dict: Record<Lang, Record<TKey, string>> = { en, vi };

/** Languages offered in the Settings picker (label shown in its own tongue). */
export const LANGUAGES: { code: Lang; label: string }[] = [
  { code: "en", label: "English" },
  { code: "vi", label: "Tiếng Việt" },
];

interface I18n {
  lang: Lang;
  t: (key: TKey) => string;
}

const I18nContext = createContext<I18n>({ lang: "en", t: (k) => dict.en[k] });

export function I18nProvider({
  lang,
  children,
}: {
  lang: Lang;
  children: ReactNode;
}) {
  useEffect(() => {
    document.documentElement.lang = lang;
  }, [lang]);

  const value = useMemo<I18n>(
    () => ({ lang, t: (key) => (dict[lang] ?? dict.en)[key] }),
    [lang],
  );

  return <I18nContext.Provider value={value}>{children}</I18nContext.Provider>;
}

export function useI18n() {
  return useContext(I18nContext);
}
