import { useCallback, useEffect, useState } from "react";
import { IconBack, IconGithub } from "../components/Icons";
import {
  eventToAccelerator,
  isValidAccelerator,
  prettyAccelerator,
} from "../lib/accelerator";
import { LANGUAGES, useI18n, type Lang } from "../lib/i18n";
import {
  checkForUpdates,
  getPasteState,
  getToggleCommand,
  getVersion,
  onEvent,
  openReleasePage,
  openUrl,
  setAutoPaste,
  setHotkey,
} from "../lib/ipc";
import type { PasteState, SessionInfo, Settings, UpdateCheck } from "../types";

/** Project author — linked from the About section. */
const AUTHOR = "datluong2409";
const AUTHOR_URL = `https://github.com/${AUTHOR}`;

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
  const { t } = useI18n();
  const [capturing, setCapturing] = useState(false);
  const [pasteState, setPasteState] = useState<PasteState | null>(null);
  const [version, setVersion] = useState<string | null>(null);
  const [checking, setChecking] = useState(false);
  const [updateResult, setUpdateResult] = useState<UpdateCheck | null>(null);
  // The manual `<app> --toggle` command shown when no auto hotkey backend exists.
  const [toggleCmd, setToggleCmd] = useState<string | null>(null);

  // Current app version for the Updates section (cheap backend call, no network).
  useEffect(() => {
    void getVersion().then(setVersion);
  }, []);

  const runUpdateCheck = useCallback(async () => {
    setChecking(true);
    try {
      setUpdateResult(await checkForUpdates());
    } finally {
      setChecking(false);
    }
  }, []);

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
        onToast(t("hotkeyChanged"));
      } else if (r.reason === "no_hotkey_backend") {
        if (settings) onLocal({ ...settings, hotkey: accel });
        onToast(t("hotkeySavedManual"));
      } else {
        onToast(t(r.reason === "invalid" ? "invalidCombo" : "registerFailed"));
      }
    },
    [settings, onLocal, onToast, t],
  );

  // Fetch the exact `<app> --toggle` command to show for manual binding on
  // sessions with no automatic hotkey backend (e.g. non-GNOME Wayland).
  useEffect(() => {
    if (sessionInfo?.hotkeyBackend !== "none") return;
    void getToggleCommand().then(setToggleCmd);
  }, [sessionInfo?.hotkeyBackend]);

  const copyToggleCommand = useCallback(async () => {
    if (!toggleCmd) return;
    try {
      await navigator.clipboard.writeText(toggleCmd);
      onToast(t("copied"));
    } catch {
      onToast(t("copyFailed"));
    }
  }, [toggleCmd, onToast, t]);

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
        onToast(t(valid.reason === "need_modifier" ? "needModifier" : "needMainKey"));
        return;
      }
      setCapturing(false);
      void applyHotkey(accel);
    }
    window.addEventListener("keydown", onKey, true);
    return () => window.removeEventListener("keydown", onKey, true);
  }, [capturing, applyHotkey, onToast, t]);

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
        {t("loading")}
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
          title={t("back")}
          className="rounded-md p-1.5 text-neutral-500 hover:bg-black/10 dark:hover:bg-white/10"
        >
          <IconBack className="h-4 w-4" />
        </button>
        <h1 className="text-sm font-semibold">{t("settings")}</h1>
      </div>

      <div className="scroll-thin flex-1 overflow-y-auto px-4 py-3">
        {/* Hotkey */}
        <section className="mb-4">
          <h2 className="mb-1 text-xs font-semibold uppercase tracking-wide text-neutral-400">
            {t("panelHotkey")}
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
            {capturing ? t("pressCombo") : prettyAccelerator(settings.hotkey)}
          </button>
          {hotkeyBackend === "gnome" && (
            <p className="mt-1 text-xs text-neutral-500 dark:text-neutral-400">
              {t("hotkeyGnomeSync")}
            </p>
          )}
          {hotkeyBackend === "none" && (
            <div className="mt-2 rounded-md border border-amber-500/30 bg-amber-500/10 px-3 py-2">
              <p className="text-xs text-amber-700 dark:text-amber-300">
                {t("hotkeyNoneExplain")}
              </p>
              <div className="mt-2 flex items-center gap-2">
                <code className="scroll-thin flex-1 overflow-x-auto whitespace-nowrap rounded bg-black/10 px-2 py-1 font-mono text-xs text-neutral-800 dark:bg-white/10 dark:text-neutral-100">
                  {toggleCmd ?? "linux-clipboard --toggle"}
                </code>
                <button
                  type="button"
                  onClick={() => void copyToggleCommand()}
                  disabled={!toggleCmd}
                  className="shrink-0 rounded-md bg-[var(--color-accent)] px-2.5 py-1 text-xs font-medium text-white disabled:opacity-60"
                >
                  {t("copy")}
                </button>
              </div>
              <p className="mt-1.5 text-xs text-amber-700/80 dark:text-amber-300/80">
                {t("hotkeyNoneHowto")}
              </p>
            </div>
          )}
        </section>

        {/* Behavior toggles */}
        <section className="mb-4 divide-y divide-black/5 dark:divide-white/10">
          <Toggle
            label={t("autoPasteLabel")}
            hint={t("autoPasteHint")}
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
              {t("needsPermission")}
            </p>
            <button
              type="button"
              onClick={() => void setAutoPaste(true)}
              className="mt-2 rounded-md bg-[var(--color-accent)] px-3 py-1 text-xs font-medium text-white"
            >
              {t("grantPermission")}
            </button>
          </section>
        )}
        {pasteState === "portal_missing" && (
          <section className="mb-4 rounded-md border border-amber-500/30 bg-amber-500/10 px-3 py-2">
            <p className="text-xs text-amber-700 dark:text-amber-300">
              {t("portalMissing")}
            </p>
          </section>
        )}

        {/* Language */}
        <section className="mb-4">
          <label className="flex items-center justify-between gap-3 py-1">
            <span className="text-sm text-neutral-800 dark:text-neutral-100">
              {t("language")}
            </span>
            <select
              value={settings.language}
              onChange={(e) =>
                onSave({ ...settings, language: e.target.value as Lang })
              }
              className="rounded-md border border-black/10 bg-white/60 px-2 py-1 text-sm outline-none dark:border-white/10 dark:bg-neutral-800"
            >
              {LANGUAGES.map((l) => (
                <option key={l.code} value={l.code}>
                  {l.label}
                </option>
              ))}
            </select>
          </label>
        </section>

        {/* History cap */}
        <section className="mb-4">
          <label className="flex items-center justify-between gap-3 py-1">
            <span className="text-sm text-neutral-800 dark:text-neutral-100">
              {t("maxItems")}
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

        {/* Updates (with author) */}
        <section className="mb-4">
          <h2 className="mb-1 text-xs font-semibold uppercase tracking-wide text-neutral-400">
            {t("updates")}
          </h2>
          <div className="flex items-center justify-between gap-3 py-1">
            <span className="text-sm text-neutral-800 dark:text-neutral-100">
              {t("author")}
            </span>
            <button
              type="button"
              onClick={() => void openUrl(AUTHOR_URL)}
              title={AUTHOR_URL}
              className="flex cursor-pointer items-center gap-1.5 text-sm font-medium text-[var(--color-accent)] hover:underline"
            >
              <IconGithub className="h-4 w-4" />
              @{AUTHOR}
            </button>
          </div>
          <div className="flex items-center justify-between gap-3 py-1">
            <span className="text-sm text-neutral-800 dark:text-neutral-100">
              {t("currentVersion")}
            </span>
            <span className="font-mono text-sm text-neutral-600 dark:text-neutral-300">
              {version ?? "…"}
            </span>
          </div>
          <button
            type="button"
            onClick={() => void runUpdateCheck()}
            disabled={checking}
            className="mt-1 w-full rounded-md border border-black/10 bg-white/60 px-3 py-2 text-sm transition hover:border-black/20 disabled:opacity-60 dark:border-white/10 dark:bg-white/10"
          >
            {checking ? t("checkingUpdates") : t("checkForUpdates")}
          </button>
          {updateResult && !checking && (
            updateResult.error ? (
              <p className="mt-2 text-xs text-red-600 dark:text-red-400">
                {t("updateCheckFailed")}
              </p>
            ) : updateResult.updateAvailable ? (
              <div className="mt-2 flex items-center justify-between gap-2">
                <p className="text-xs font-medium text-[var(--color-accent)]">
                  {t("updateAvailable")} {updateResult.latestVersion}
                </p>
                <button
                  type="button"
                  onClick={() => void openReleasePage(updateResult.releaseUrl)}
                  className="shrink-0 rounded-md bg-[var(--color-accent)] px-3 py-1 text-xs font-medium text-white"
                >
                  {t("downloadUpdate")}
                </button>
              </div>
            ) : (
              <p className="mt-2 text-xs text-neutral-500 dark:text-neutral-400">
                {t("upToDate")}
              </p>
            )
          )}
        </section>

        {/* Session info */}
        <section className="mt-2 rounded-md bg-black/5 px-3 py-2 text-xs text-neutral-500 dark:bg-white/5 dark:text-neutral-400">
          <div className="flex justify-between">
            <span>{t("displaySession")}</span>
            <span className="font-mono uppercase">{sessionInfo?.kind ?? "?"}</span>
          </div>
          <div className="flex justify-between">
            <span>{t("hotkeyMechanism")}</span>
            <span className="font-mono">{sessionInfo?.hotkeyBackend ?? "?"}</span>
          </div>
          <div className="flex justify-between">
            <span>{t("autoPasteBackend")}</span>
            <span className="font-mono">{sessionInfo?.autoPasteBackend ?? "?"}</span>
          </div>
        </section>
      </div>
    </div>
  );
}
