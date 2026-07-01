import { useCallback, useEffect, useState } from "react";
import { IconBack } from "../components/Icons";
import {
  eventToAccelerator,
  isValidAccelerator,
  prettyAccelerator,
} from "../lib/accelerator";
import {
  clearHistory,
  configureGnomeShortcut,
  removeGnomeShortcut,
  setHotkey,
} from "../lib/ipc";
import type { SessionInfo, Settings } from "../types";

interface Props {
  settings: Settings | null;
  sessionInfo: SessionInfo | null;
  onSave: (s: Settings) => void;
  onLocal: (s: Settings) => void;
  onBack: () => void;
  onToast: (msg: string) => void;
}

function Toggle({
  checked,
  onChange,
  label,
  hint,
}: {
  checked: boolean;
  onChange: (v: boolean) => void;
  label: string;
  hint?: string;
}) {
  return (
    <button
      type="button"
      onClick={() => onChange(!checked)}
      className="flex w-full items-center justify-between gap-3 py-2 text-left"
    >
      <span>
        <span className="block text-sm text-neutral-800 dark:text-neutral-100">
          {label}
        </span>
        {hint && (
          <span className="block text-xs text-neutral-500 dark:text-neutral-400">
            {hint}
          </span>
        )}
      </span>
      <span
        className={[
          "relative h-5 w-9 shrink-0 rounded-full transition",
          checked
            ? "bg-[var(--color-accent)]"
            : "bg-neutral-300 dark:bg-neutral-600",
        ].join(" ")}
      >
        <span
          className={[
            "absolute top-0.5 h-4 w-4 rounded-full bg-white transition-all",
            checked ? "left-4" : "left-0.5",
          ].join(" ")}
        />
      </span>
    </button>
  );
}

export function SettingsView({
  settings,
  sessionInfo,
  onSave,
  onLocal,
  onBack,
  onToast,
}: Props) {
  const [capturing, setCapturing] = useState(false);

  const applyHotkey = useCallback(
    async (accel: string) => {
      const r = await setHotkey(accel);
      if (r.ok) {
        if (settings) onLocal({ ...settings, hotkey: accel });
        onToast("Đã đổi phím tắt");
      } else if (r.reason === "wayland_use_gnome") {
        if (settings) onLocal({ ...settings, hotkey: accel });
        onToast("Wayland: dùng nút 'Cấu hình GNOME shortcut' bên dưới");
      } else {
        onToast(
          r.reason === "invalid"
            ? "Tổ hợp không hợp lệ"
            : "Không đăng ký được (có thể đã bị dùng)",
        );
      }
    },
    [settings, onLocal, onToast],
  );

  // Capture a key combo while recording.
  useEffect(() => {
    if (!capturing) return;
    function onKey(e: KeyboardEvent) {
      e.preventDefault();
      e.stopPropagation();
      if (e.key === "Escape") {
        setCapturing(false);
        return;
      }
      const accel = eventToAccelerator(e);
      if (!accel) return; // only modifiers held so far
      const valid = isValidAccelerator(accel);
      if (!valid.ok) {
        onToast(valid.reason ?? "Tổ hợp không hợp lệ");
        return;
      }
      setCapturing(false);
      void applyHotkey(accel);
    }
    window.addEventListener("keydown", onKey, true);
    return () => window.removeEventListener("keydown", onKey, true);
  }, [capturing, applyHotkey, onToast]);

  // Escape leaves settings (unless mid-capture).
  useEffect(() => {
    function onKey(e: KeyboardEvent) {
      if (e.key === "Escape" && !capturing) {
        e.preventDefault();
        onBack();
      }
    }
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [capturing, onBack]);

  if (!settings) {
    return (
      <div className="flex h-full items-center justify-center text-sm text-neutral-500">
        Đang tải…
      </div>
    );
  }

  const isWayland = sessionInfo?.kind === "wayland";

  return (
    <div className="flex h-full flex-col">
      <div
        data-tauri-drag-region
        className="flex items-center gap-2 border-b border-black/5 px-3 py-2.5 dark:border-white/10"
      >
        <button
          type="button"
          onClick={onBack}
          title="Quay lại"
          className="rounded-md p-1.5 text-neutral-500 hover:bg-black/10 dark:hover:bg-white/10"
        >
          <IconBack className="h-4 w-4" />
        </button>
        <h1 className="text-sm font-semibold">Cài đặt</h1>
      </div>

      <div className="scroll-thin flex-1 overflow-y-auto px-4 py-3">
        {/* Hotkey */}
        <section className="mb-4">
          <h2 className="mb-1 text-xs font-semibold uppercase tracking-wide text-neutral-400">
            Phím tắt mở bảng
          </h2>
          <button
            type="button"
            onClick={() => setCapturing(true)}
            className={[
              "w-full rounded-md border px-3 py-2 text-center text-sm transition",
              capturing
                ? "border-[var(--color-accent)] bg-[var(--color-accent-soft)]/40 text-[var(--color-accent)]"
                : "border-black/10 bg-white/60 hover:border-black/20 dark:border-white/10 dark:bg-white/10",
            ].join(" ")}
          >
            {capturing
              ? "Nhấn tổ hợp phím… (Esc để huỷ)"
              : prettyAccelerator(settings.hotkey)}
          </button>
          {isWayland && (
            <p className="mt-1 text-xs text-amber-600 dark:text-amber-400">
              Trên Wayland, phím tắt trong app không hoạt động — hãy dùng GNOME
              shortcut bên dưới.
            </p>
          )}
        </section>

        {/* Behavior toggles */}
        <section className="mb-4 divide-y divide-black/5 dark:divide-white/10">
          <Toggle
            label="Tự động dán (auto-paste)"
            hint="Dán thẳng vào app đang mở khi chọn 1 mục"
            checked={settings.autoPaste}
            onChange={(v) => onSave({ ...settings, autoPaste: v })}
          />
          <Toggle
            label="Ghi lại ảnh"
            hint="Lưu cả ảnh/screenshot vào lịch sử"
            checked={settings.captureImages}
            onChange={(v) => onSave({ ...settings, captureImages: v })}
          />
          <Toggle
            label="Ẩn khi mất focus"
            hint="Đóng bảng khi bấm ra ngoài"
            checked={settings.hideOnBlur}
            onChange={(v) => onSave({ ...settings, hideOnBlur: v })}
          />
          <Toggle
            label="Khởi động cùng hệ thống"
            hint="Chạy nền khi đăng nhập"
            checked={settings.autostart}
            onChange={(v) => onSave({ ...settings, autostart: v })}
          />
        </section>

        {/* History cap */}
        <section className="mb-4">
          <label className="flex items-center justify-between gap-3 py-1">
            <span className="text-sm text-neutral-800 dark:text-neutral-100">
              Số mục tối đa
            </span>
            <input
              type="number"
              min={5}
              max={500}
              value={settings.historyCap}
              onChange={(e) => {
                const n = Math.max(5, Math.min(500, Number(e.target.value) || 50));
                onSave({ ...settings, historyCap: n });
              }}
              className="w-20 rounded-md border border-black/10 bg-white/60 px-2 py-1 text-right text-sm outline-none dark:border-white/10 dark:bg-white/10"
            />
          </label>
        </section>

        {/* GNOME shortcut helper */}
        <section className="mb-4">
          <h2 className="mb-1 text-xs font-semibold uppercase tracking-wide text-neutral-400">
            GNOME shortcut {isWayland ? "(khuyên dùng)" : "(tuỳ chọn)"}
          </h2>
          <p className="mb-2 text-xs text-neutral-500 dark:text-neutral-400">
            Tạo custom shortcut trong GNOME chạy lệnh mở bảng — hoạt động cả trên
            Wayland.
          </p>
          <div className="flex gap-2">
            <button
              type="button"
              onClick={async () => {
                const r = await configureGnomeShortcut(settings.hotkey);
                if (r.ok) {
                  onLocal({ ...settings, gnomeShortcutConfigured: true });
                  onToast("Đã tạo GNOME shortcut");
                } else {
                  onToast("Không tạo được — thử Settings → Keyboard thủ công");
                }
              }}
              className="flex-1 rounded-md bg-[var(--color-accent)] px-3 py-2 text-sm font-medium text-white hover:opacity-90"
            >
              Cấu hình GNOME shortcut
            </button>
            {settings.gnomeShortcutConfigured && (
              <button
                type="button"
                onClick={async () => {
                  const r = await removeGnomeShortcut();
                  if (r.ok) {
                    onLocal({ ...settings, gnomeShortcutConfigured: false });
                    onToast("Đã gỡ GNOME shortcut");
                  }
                }}
                className="rounded-md border border-black/10 px-3 py-2 text-sm hover:bg-black/5 dark:border-white/10 dark:hover:bg-white/10"
              >
                Gỡ
              </button>
            )}
          </div>
        </section>

        {/* Danger zone */}
        <section className="mb-4">
          <button
            type="button"
            onClick={async () => {
              await clearHistory(false);
              onToast("Đã xoá toàn bộ lịch sử");
            }}
            className="w-full rounded-md border border-red-500/30 px-3 py-2 text-sm text-red-600 hover:bg-red-500/10 dark:text-red-400"
          >
            Xoá toàn bộ lịch sử (kể cả mục đã ghim)
          </button>
        </section>

        {/* Session info */}
        <section className="mt-2 rounded-md bg-black/5 px-3 py-2 text-xs text-neutral-500 dark:bg-white/5 dark:text-neutral-400">
          <div className="flex justify-between">
            <span>Phiên hiển thị</span>
            <span className="font-mono uppercase">{sessionInfo?.kind ?? "?"}</span>
          </div>
          <div className="flex justify-between">
            <span>Auto-paste backend</span>
            <span className="font-mono">{sessionInfo?.autoPasteBackend ?? "?"}</span>
          </div>
        </section>
      </div>
    </div>
  );
}
