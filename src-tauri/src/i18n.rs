//! Backend-side UI strings (tray menu + native dialogs) localized to the user's
//! chosen language. The React frontend has its own i18n (`src/lib/i18n.tsx`);
//! this module covers only the text Rust renders directly. English is the
//! default and the fallback for any unrecognized language code.

/// UI language, mirrored from `Settings.language` ("en" | "vi").
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Lang {
    En,
    Vi,
}

impl Lang {
    pub fn from_code(code: &str) -> Self {
        match code {
            "vi" => Lang::Vi,
            _ => Lang::En,
        }
    }

    /// Pick the variant for this language (English is the fallback).
    fn pick(self, en: &'static str, vi: &'static str) -> &'static str {
        match self {
            Lang::En => en,
            Lang::Vi => vi,
        }
    }

    // --- Tray menu ---

    pub fn tray_show(self) -> &'static str {
        self.pick("Show clipboard", "Hiện clipboard")
    }

    pub fn tray_settings(self) -> &'static str {
        self.pick("Settings", "Cài đặt")
    }

    pub fn tray_check_updates(self) -> &'static str {
        self.pick("Check for updates", "Kiểm tra cập nhật")
    }

    pub fn tray_quit(self) -> &'static str {
        self.pick("Quit", "Thoát")
    }

    pub fn tray_auto_paste_portal_missing(self) -> &'static str {
        self.pick("Auto-paste: portal missing ⚠", "Auto-paste: thiếu portal ⚠")
    }

    pub fn tray_auto_paste_needs_permission(self) -> &'static str {
        self.pick(
            "Auto-paste: needs permission ⚠",
            "Auto-paste: cần cấp quyền ⚠",
        )
    }

    pub fn tray_auto_paste_on(self) -> &'static str {
        self.pick("Auto-paste: On ✓", "Auto-paste: Bật ✓")
    }

    pub fn tray_auto_paste_off(self) -> &'static str {
        self.pick("Auto-paste: Off", "Auto-paste: Tắt")
    }

    // --- Native dialogs ---

    pub fn portal_missing_title(self) -> &'static str {
        self.pick("xdg-desktop-portal missing", "Thiếu xdg-desktop-portal")
    }

    pub fn portal_missing_body(self) -> &'static str {
        self.pick(
            "This machine has no RemoteDesktop portal, so auto-paste isn't \
             possible on Wayland.\n\n\
             Install the package for your desktop:\n  \
             • GNOME: xdg-desktop-portal-gnome\n  \
             • KDE:   xdg-desktop-portal-kde\n\n\
             For example on Ubuntu/Debian:\n  \
             sudo apt install xdg-desktop-portal-gnome\n\n\
             The content is still copied — you can paste it yourself with Ctrl+V.",
            "Máy này chưa có portal RemoteDesktop nên không thể tự động dán trên Wayland.\n\n\
             Hãy cài gói tương ứng với desktop của bạn:\n  \
             • GNOME: xdg-desktop-portal-gnome\n  \
             • KDE:   xdg-desktop-portal-kde\n\n\
             Ví dụ trên Ubuntu/Debian:\n  \
             sudo apt install xdg-desktop-portal-gnome\n\n\
             Nội dung vẫn được copy — bạn có thể tự dán bằng Ctrl+V.",
        )
    }

    pub fn enable_paste_title(self) -> &'static str {
        self.pick("Enable auto-paste?", "Bật auto-paste?")
    }

    pub fn enable_paste_body(self) -> &'static str {
        self.pick(
            "On Wayland, auto-pasting into other apps needs a one-time Remote \
             Desktop permission.\n\n\
             Enable it now? If you skip, the content is still copied — just \
             paste it yourself with Ctrl+V.",
            "Trên Wayland, để tự động dán vào ứng dụng bạn cần cấp quyền Remote \
             Desktop một lần.\n\n\
             Bật ngay? Nếu để sau, nội dung vẫn đã được copy — bạn tự dán bằng \
             Ctrl+V.",
        )
    }

    pub fn enable_paste_now(self) -> &'static str {
        self.pick("Enable now", "Bật ngay")
    }

    pub fn enable_paste_later(self) -> &'static str {
        self.pick("Later", "Để sau")
    }

    pub fn copied_title(self) -> &'static str {
        self.pick("Copied", "Đã copy")
    }

    pub fn copied_body(self) -> &'static str {
        self.pick(
            "The content has been copied to the clipboard. You can enable \
             auto-paste later in Settings, or from the system tray menu.",
            "Nội dung đã được copy vào clipboard. Bạn có thể bật auto-paste sau \
             này trong Settings, hoặc ở menu khay hệ thống (tray).",
        )
    }

    // --- Update check dialogs (tray) ---

    pub fn update_available_title(self) -> &'static str {
        self.pick("Update available", "Có bản cập nhật")
    }

    /// Body for "a newer version exists" (interpolates both versions).
    pub fn update_available_body(self, latest: &str, current: &str) -> String {
        match self {
            Lang::En => format!(
                "A new version {latest} is available (you have {current}).\n\n\
                 Open the download page?"
            ),
            Lang::Vi => format!(
                "Đã có phiên bản mới {latest} (bạn đang dùng {current}).\n\n\
                 Mở trang tải về?"
            ),
        }
    }

    pub fn update_open(self) -> &'static str {
        self.pick("Open", "Mở")
    }

    pub fn update_later(self) -> &'static str {
        self.pick("Later", "Để sau")
    }

    pub fn update_up_to_date_title(self) -> &'static str {
        self.pick("You're up to date", "Bạn đang ở bản mới nhất")
    }

    pub fn update_up_to_date_body(self, current: &str) -> String {
        match self {
            Lang::En => format!("You're on the latest version ({current})."),
            Lang::Vi => format!("Bạn đang dùng phiên bản mới nhất ({current})."),
        }
    }

    pub fn update_error_title(self) -> &'static str {
        self.pick("Update check failed", "Kiểm tra cập nhật thất bại")
    }

    pub fn update_error_body(self) -> &'static str {
        self.pick(
            "Couldn't check for updates. Please check your internet connection \
             and try again.",
            "Không thể kiểm tra cập nhật. Vui lòng kiểm tra kết nối mạng và thử lại.",
        )
    }
}
