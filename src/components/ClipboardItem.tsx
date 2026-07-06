import { useI18n } from "../lib/i18n";
import { assetUrl } from "../lib/ipc";
import type { Clip } from "../types";
import {
  IconImage,
  IconPin,
  IconPinFilled,
  IconPlainText,
  IconTrash,
} from "./Icons";

interface Props {
  clip: Clip;
  selected: boolean;
  onPaste: () => void;
  /** Paste without the captured formatting (only shown for rich-text clips). */
  onPastePlain: () => void;
  onTogglePin: () => void;
  onDelete: () => void;
}

export function ClipboardItem({
  clip,
  selected,
  onPaste,
  onPastePlain,
  onTogglePin,
  onDelete,
}: Props) {
  const { t } = useI18n();
  return (
    <div
      onClick={onPaste}
      data-selected={selected || undefined}
      className={[
        "group relative flex h-24 cursor-pointer items-center overflow-hidden rounded-lg border p-3 transition",
        selected
          ? "border-[var(--color-accent)] bg-[var(--color-accent-soft)]/50 ring-1 ring-[var(--color-accent)] dark:bg-[var(--color-accent)]/20"
          : "border-black/5 bg-white hover:border-neutral-400 dark:border-white/5 dark:bg-white/5 dark:hover:border-neutral-500 dark:hover:bg-white/10",
      ].join(" ")}
    >
      {clip.kind === "image" && clip.thumbPath ? (
        <img
          src={assetUrl(clip.thumbPath)}
          alt="clipboard"
          className="mx-auto max-h-full max-w-full rounded object-contain"
        />
      ) : (
        <p className="clip-preview line-clamp-3 w-full self-start whitespace-pre-wrap text-sm text-neutral-800 dark:text-neutral-100">
          {clip.content}
        </p>
      )}

      <div className="absolute right-1.5 top-1.5 flex gap-1 rounded-lg bg-[var(--color-card)] p-0.5 opacity-0 shadow-sm ring-1 ring-black/5 transition group-hover:opacity-100 dark:bg-[var(--color-card-dark)] dark:ring-white/10">
        {clip.kind === "text" && clip.html ? (
          <button
            type="button"
            title={t("pastePlain")}
            onClick={(e) => {
              e.stopPropagation();
              onPastePlain();
            }}
            className="rounded p-1 text-neutral-500 hover:bg-black/10 hover:text-[var(--color-accent)] dark:hover:bg-white/10"
          >
            <IconPlainText className="h-4 w-4" />
          </button>
        ) : null}
        <button
          type="button"
          title={clip.pinned ? t("unpin") : t("pin")}
          onClick={(e) => {
            e.stopPropagation();
            onTogglePin();
          }}
          className="rounded p-1 text-neutral-500 hover:bg-black/10 hover:text-[var(--color-accent)] dark:hover:bg-white/10"
        >
          {clip.pinned ? (
            <IconPinFilled className="h-4 w-4 text-[var(--color-accent)]" />
          ) : (
            <IconPin className="h-4 w-4" />
          )}
        </button>
        <button
          type="button"
          title={t("delete")}
          onClick={(e) => {
            e.stopPropagation();
            onDelete();
          }}
          className="rounded p-1 text-neutral-500 hover:bg-black/10 hover:text-red-500 dark:hover:bg-white/10"
        >
          <IconTrash className="h-4 w-4" />
        </button>
      </div>

      {clip.kind === "image" && clip.width ? (
        <span className="absolute bottom-1.5 left-1.5 flex items-center gap-1 rounded bg-black/40 px-1 py-0.5 text-[10px] text-white">
          <IconImage className="h-3 w-3" />
          {clip.width}×{clip.height}
        </span>
      ) : null}

      {clip.kind === "text" && clip.html ? (
        <span
          title={t("formatted")}
          className="absolute bottom-1.5 left-1.5 rounded bg-black/40 px-1.5 py-0.5 text-[10px] font-semibold leading-none text-white"
        >
          <span className="italic underline">A</span>
        </span>
      ) : null}
    </div>
  );
}
