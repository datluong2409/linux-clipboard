import { assetUrl } from "../lib/ipc";
import type { Clip } from "../types";
import { IconImage, IconPin, IconPinFilled, IconTrash } from "./Icons";

interface Props {
  clip: Clip;
  selected: boolean;
  onPaste: () => void;
  onTogglePin: () => void;
  onDelete: () => void;
}

export function ClipboardItem({
  clip,
  selected,
  onPaste,
  onTogglePin,
  onDelete,
}: Props) {
  return (
    <div
      onClick={onPaste}
      className={[
        "group relative cursor-pointer rounded-lg border p-2.5 transition",
        selected
          ? "border-[var(--color-accent)] bg-[var(--color-accent-soft)]/50 ring-1 ring-[var(--color-accent)]"
          : "border-black/5 bg-white hover:border-black/10 dark:border-white/5 dark:bg-white/5 dark:hover:bg-white/10",
      ].join(" ")}
    >
      {clip.kind === "image" && clip.thumbPath ? (
        <img
          src={assetUrl(clip.thumbPath)}
          alt="clipboard"
          className="max-h-28 w-auto rounded object-contain"
        />
      ) : (
        <p className="clip-preview line-clamp-4 whitespace-pre-wrap text-sm text-neutral-800 dark:text-neutral-100">
          {clip.content}
        </p>
      )}

      <div className="absolute right-1.5 top-1.5 flex gap-1 opacity-0 transition group-hover:opacity-100">
        <button
          type="button"
          title={clip.pinned ? "Bỏ ghim" : "Ghim"}
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
          title="Xoá"
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
    </div>
  );
}
