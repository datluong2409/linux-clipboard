import { useCallback, useEffect, useState } from "react";
import { IconBack } from "../components/Icons";
import {
  eventToAccelerator,
  isValidAccelerator,
  prettyAccelerator,
} from "../lib/accelerator";
import {
  clearHistory,
  getPasteState,
  onEvent,
  setAutoPaste,
  setHotkey,
} from "../lib/ipc";
import type { PasteState, SessionInfo, Settings } from "../types";

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
  const [pasteState, setPasteState] = useState<PasteState | null>(null);

  // Mirror the tray's live auto-paste state (grant/portal status). Re-read on
  // `settings-updated`, which both the toggle and the grant flow emit.
  useEffect(() => {
    void getPasteState().then(setPasteState);
    const un = onEvent(
      "settings-updated",
      () => void getPasteState().then(setPasteState),
    );
    return () => {
      void un.then((u) => u());
    };
  }, []);

  const applyHotkey = useCallback(
    async (accel: string) => {
      const r = await setHotkey(accel);
      if (r.ok) {
        if (settings) onLocal({ ...settings, hotkey: accel });
        onToast("Đã đổi phím tắt");
      } else if (r.reason === "no_hotkey_backend") {
        if (settings) onLocal({ ...settings, hotkey: accel });
        onToast("Đã lưu phím tắt — môi trường này cần tạo shortcut thủ công");
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

  const hotkeyBackend = sessionInfo?.hotkeyBackend;

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
          {hotkeyBackend === "gnome" && (
            <p className="mt-1 text-xs text-neutral-500 dark:text-neutral-400">
              Tự động đồng bộ với shortcut hệ thống GNOME (áp dụng ngay, kể cả
              trên Wayland).
            </p>
          )}
          {hotkeyBackend === "none" && (
            <p className="mt-1 text-xs text-amber-600 dark:text-amber-400">
              Môi trường này (Wayland ngoài GNOME) không cho app tự đăng ký phím
              tắt. Phím tắt vẫn được lưu — hãy tạo shortcut trong cài đặt bàn
              phím của desktop, chạy lệnh{" "}
              <code className="rounded bg-black/10 px-1 dark:bg-white/10">
                linux-clipboard --toggle
              </code>
              .
            </p>
          )}
        </section>

        {/* Behavior toggles */}
        <section className="mb-4 divide-y divide-black/5 dark:divide-white/10">
          <Toggle
            label="Tự động dán (auto-paste)"
            hint="Dán thẳng vào app đang mở khi chọn 1 mục"
            checked={settings.autoPaste}
            onChange={(v) => {
              // Optimistic UI; the backend runs the same state machine as the
              // tray (grant flow on Wayland) and echoes back via settings-updated.
              onLocal({ ...settings, autoPaste: v });
              void setAutoPaste(v);
            }}
          />
        </section>

        {/* Auto-paste portal status (Wayland), mirroring the tray's states. */}
        {pasteState === "needs_permission" && (
          <section className="mb-4 rounded-md border border-amber-500/30 bg-amber-500/10 px-3 py-2">
            <p className="text-xs text-amber-700 dark:text-amber-300">
              Cần cấp quyền Remote Desktop (một lần) để tự động dán trên Wayland.
            </p>
            <button
              type="button"
              onClick={() => void setAutoPaste(true)}
              className="mt-2 rounded-md bg-[var(--color-accent)] px-3 py-1 text-xs font-medium text-white"
            >
              Cấp quyền
            </button>
          </section>
        )}
        {pasteState === "portal_missing" && (
          <section className="mb-4 rounded-md border border-amber-500/30 bg-amber-500/10 px-3 py-2">
            <p className="text-xs text-amber-700 dark:text-amber-300">
              Chưa có backend xdg-desktop-portal (gnome/kde) nên không thể tự
              động dán. Nội dung vẫn được copy để bạn tự dán bằng Ctrl+V.
            </p>
          </section>
        )}

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
                const n = Math.max(5, Math.min(500, Number(e.target.value) || 25));
                onSave({ ...settings, historyCap: n });
              }}
              className="w-20 rounded-md border border-black/10 bg-white/60 px-2 py-1 text-right text-sm outline-none dark:border-white/10 dark:bg-white/10"
            />
          </label>
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
            <span>Cơ chế phím tắt</span>
            <span className="font-mono">{sessionInfo?.hotkeyBackend ?? "?"}</span>
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
