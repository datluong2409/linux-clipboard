import { useEffect, useRef, useState } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { ClipboardPanel } from "./components/ClipboardPanel";
import { SettingsView } from "./views/SettingsView";
import { Toast } from "./components/Toast";
import { useSettings } from "./hooks/useSettings";
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

  // Hide on focus loss (Win+V behavior). Ignore the transient blur right after
  // we grab focus on show, and never auto-hide while the Settings view is open.
  useEffect(() => {
    const win = getCurrentWindow();
    let ignoreUntil = 0;
    const p = win.onFocusChanged(({ payload: focused }) => {
      if (focused) {
        ignoreUntil = Date.now() + 300;
        return;
      }
      if (Date.now() < ignoreUntil) return;
      if (viewRef.current !== "settings") {
        void win.hide();
      }
    });
    return () => {
      void p.then((u) => u());
    };
  }, []);

  function showToast(msg: string) {
    setToast(msg);
    window.setTimeout(() => setToast(null), 2200);
  }

  return (
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
  );
}
