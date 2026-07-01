import { useCallback, useEffect, useState } from "react";
import {
  getSessionInfo,
  getSettings,
  onEvent,
  saveSettings as saveSettingsIpc,
} from "../lib/ipc";
import type { SessionInfo, Settings } from "../types";

export function useSettings() {
  const [settings, setSettings] = useState<Settings | null>(null);
  const [sessionInfo, setSessionInfo] = useState<SessionInfo | null>(null);

  useEffect(() => {
    void getSettings().then(setSettings);
    void getSessionInfo().then(setSessionInfo);
    const un = onEvent("settings-updated", () => void getSettings().then(setSettings));
    return () => {
      void un.then((u) => u());
    };
  }, []);

  /** Update local state AND persist to the backend (applies side effects). */
  const save = useCallback(async (next: Settings) => {
    setSettings(next);
    await saveSettingsIpc(next);
  }, []);

  return { settings, sessionInfo, setSettings, save };
}
