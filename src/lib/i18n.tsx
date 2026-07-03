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
  hotkeyNoneBefore:
    "This environment (Wayland outside GNOME) can't let the app register a global hotkey. The hotkey is still saved — create a shortcut in your desktop's keyboard settings that runs ",
  hotkeyNoneAfter: ".",

  // Settings — auto-paste
  autoPasteLabel: "Auto-paste",
  autoPasteHint: "Paste straight into the active app when you pick an item",
  needsPermission:
    "Remote Desktop permission (one-time) is required to auto-paste on Wayland.",
  grantPermission: "Grant permission",
  portalMissing:
    "No xdg-desktop-portal backend (gnome/kde), so auto-paste isn't possible. The content is still copied for you to paste with Ctrl+V.",

  // Settings — preferences
  language: "Language",
  maxItems: "Max items",

  // Settings — danger zone
  clearAllHistory: "Clear all history (including pinned)",
  clearedAllHistory: "Cleared all history",

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

  emptyNoResults: "Không tìm thấy kết quả",
  emptyNothing: "Chưa có gì trong lịch sử clipboard",
  emptyHint: "Copy (Ctrl+C) ở bất kỳ đâu để bắt đầu.",

  back: "Quay lại",
  loading: "Đang tải…",

  panelHotkey: "Phím tắt mở bảng",
  pressCombo: "Nhấn tổ hợp phím… (Esc để huỷ)",
  hotkeyGnomeSync:
    "Tự động đồng bộ với shortcut hệ thống GNOME (áp dụng ngay, kể cả trên Wayland).",
  hotkeyNoneBefore:
    "Môi trường này (Wayland ngoài GNOME) không cho app tự đăng ký phím tắt. Phím tắt vẫn được lưu — hãy tạo shortcut trong cài đặt bàn phím của desktop, chạy lệnh ",
  hotkeyNoneAfter: ".",

  autoPasteLabel: "Tự động dán (auto-paste)",
  autoPasteHint: "Dán thẳng vào app đang mở khi chọn 1 mục",
  needsPermission:
    "Cần cấp quyền Remote Desktop (một lần) để tự động dán trên Wayland.",
  grantPermission: "Cấp quyền",
  portalMissing:
    "Chưa có backend xdg-desktop-portal (gnome/kde) nên không thể tự động dán. Nội dung vẫn được copy để bạn tự dán bằng Ctrl+V.",

  language: "Ngôn ngữ",
  maxItems: "Số mục tối đa",

  clearAllHistory: "Xoá toàn bộ lịch sử (kể cả mục đã ghim)",
  clearedAllHistory: "Đã xoá toàn bộ lịch sử",

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
