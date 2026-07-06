import { useEffect, useRef, useState } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { ClipboardPanel } from "./components/ClipboardPanel";
import { SettingsView } from "./views/SettingsView";
import { Toast } from "./components/Toast";
import { useSettings } from "./hooks/useSettings";
import { I18nProvider } from "./lib/i18n";
import { onEvent } from "./lib/ipc";

type View = "panel" | "settings";

export default function App() {
  const [view, setView] = useState<View>("panel");
  const [shownTick, setShownTick] = useState(0);
  const [toast, setToast] = useState<string | null>(null);
  const { settings, sessionInfo, setSettings, save } = useSettings();

  const viewRef = useRef<View>("panel");
  useEffect(() => {
    viewRef.current = view;
  }, [view]);

  // Backend-driven navigation.
  useEffect(() => {
    const subs = [
      onEvent("panel-shown", () => {
        setView("panel");
        setShownTick((t) => t + 1);
      }),
      onEvent("open-settings", () => setView("settings")),
    ];
    return () => {
      subs.forEach((p) => void p.then((u) => u()));
    };
  }, []);

  // Auto-hide (Win+V behavior), made robust to Wayland's focus model.
  //
  // Primary path — hide on focus loss. Ignore the transient blur right after we
  // grab focus on show, and never auto-hide while Settings is open.
  //
  // Fallback path — on Wayland the panel shown via the global hotkey usually
  // never *receives* keyboard focus (compositor focus-stealing prevention), so
  // no focus-loss event ever fires and the panel would stay open forever. Once
  // the cursor has been over the panel, treat it leaving the window as "done"
  // and hide. This is gated on the window being unfocused, so on X11 (and any
  // Wayland setup that does grant focus) it stays inert and the focus path alone
  // governs dismissal — no "vanishes when the mouse wanders off" surprise there.
  useEffect(() => {
    const win = getCurrentWindow();
    const root = document.documentElement;
    let ignoreUntil = 0;
    let pointerInside = false;
    let leaveTimer = 0;

    const cancelLeave = () => {
      if (leaveTimer) {
        window.clearTimeout(leaveTimer);
        leaveTimer = 0;
      }
    };
    const markInside = () => {
      pointerInside = true;
      cancelLeave();
    };

    const focusSub = win.onFocusChanged(({ payload: focused }) => {
      if (focused) {
        ignoreUntil = Date.now() + 300;
        cancelLeave(); // real focus now governs dismissal
        return;
      }
      if (Date.now() < ignoreUntil) return;
      if (viewRef.current !== "settings") {
        void win.hide();
      }
    });

    const onPointerLeave = () => {
      // Only the unfocused-panel case; when focused, the focus path handles it.
      if (!pointerInside || viewRef.current === "settings" || document.hasFocus()) {
        return;
      }
      cancelLeave();
      leaveTimer = window.setTimeout(() => {
        if (viewRef.current !== "settings" && !document.hasFocus()) {
          void win.hide();
        }
      }, 300);
    };

    // Reset the "cursor has visited" flag on every fresh show.
    const shownSub = onEvent("panel-shown", () => {
      pointerInside = false;
      cancelLeave();
    });

    root.addEventListener("mousemove", markInside);
    root.addEventListener("mouseenter", markInside);
    root.addEventListener("mouseleave", onPointerLeave);

    return () => {
      cancelLeave();
      root.removeEventListener("mousemove", markInside);
      root.removeEventListener("mouseenter", markInside);
      root.removeEventListener("mouseleave", onPointerLeave);
      void focusSub.then((u) => u());
      void shownSub.then((u) => u());
    };
  }, []);

  function showToast(msg: string) {
    setToast(msg);
    window.setTimeout(() => setToast(null), 2200);
  }

  return (
    <I18nProvider lang={settings?.language ?? "en"}>
      <div className="relative flex h-full w-full flex-col overflow-hidden rounded-t-xl border border-black/10 bg-[var(--color-panel)] text-neutral-900 shadow-2xl dark:border-white/10 dark:bg-[var(--color-panel-dark)] dark:text-neutral-100">
        {view === "panel" ? (
          <ClipboardPanel
            refreshKey={shownTick}
            onToast={showToast}
            onOpenSettings={() => setView("settings")}
          />
        ) : (
          <SettingsView
            settings={settings}
            sessionInfo={sessionInfo}
            onSave={save}
            onLocal={setSettings}
            onBack={() => setView("panel")}
            onToast={showToast}
          />
        )}
        {toast && <Toast message={toast} />}
      </div>
    </I18nProvider>
  );
}
