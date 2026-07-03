import { useCallback, useEffect, useRef, useState } from "react";
import { ClipboardItem } from "./ClipboardItem";
import { EmptyState } from "./EmptyState";
import { SearchBar } from "./SearchBar";
import { IconSettings, IconTrash } from "./Icons";
import { useHistory } from "../hooks/useHistory";
import {
  clearHistory,
  deleteItem,
  hidePanel,
  pasteItem,
  pinItem,
} from "../lib/ipc";
import type { Clip } from "../types";

interface Props {
  refreshKey: number;
  onToast: (msg: string) => void;
  onOpenSettings: () => void;
}

export function ClipboardPanel({ refreshKey, onToast, onOpenSettings }: Props) {
  const [query, setQuery] = useState("");
  const { items, reload } = useHistory(query, refreshKey);
  const [sel, setSel] = useState(0);
  const searchRef = useRef<HTMLInputElement>(null);

  useEffect(() => {
    setSel(0);
  }, [query, items.length]);

  // Focus search each time the panel is (re)shown.
  useEffect(() => {
    const t = setTimeout(() => searchRef.current?.focus(), 30);
    return () => clearTimeout(t);
  }, [refreshKey]);

  const doPaste = useCallback(
    async (id: number) => {
      const r = await pasteItem(id);
      if (r?.reason === "copied") onToast("Đã copy — nhấn Ctrl+V để dán");
    },
    [onToast],
  );

  const onTogglePin = useCallback(
    async (id: number, pinned: boolean) => {
      await pinItem(id, pinned);
      void reload();
    },
    [reload],
  );

  const onDelete = useCallback(
    async (id: number) => {
      await deleteItem(id);
      void reload();
    },
    [reload],
  );

  // Keyboard navigation over the flat, pinned-first list.
  useEffect(() => {
    function onKey(e: KeyboardEvent) {
      if (e.key === "Escape") {
        e.preventDefault();
        void hidePanel();
      } else if (e.key === "ArrowDown") {
        e.preventDefault();
        setSel((s) => Math.min(s + 1, items.length - 1));
      } else if (e.key === "ArrowUp") {
        e.preventDefault();
        setSel((s) => Math.max(s - 1, 0));
      } else if (e.key === "Enter") {
        const it = items[sel];
        if (it) {
          e.preventDefault();
          void doPaste(it.id);
        }
      }
    }
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [items, sel, doPaste]);

  const pinned = query ? [] : items.filter((i) => i.pinned);
  const rest = query ? items : items.filter((i) => !i.pinned);

  const renderItem = (clip: Clip, index: number) => (
    <ClipboardItem
      key={clip.id}
      clip={clip}
      selected={index === sel}
      onPaste={() => void doPaste(clip.id)}
      onTogglePin={() => void onTogglePin(clip.id, !clip.pinned)}
      onDelete={() => void onDelete(clip.id)}
    />
  );

  return (
    <div className="flex h-full flex-col">
      <div
        data-tauri-drag-region
        className="flex items-center gap-2 border-b border-black/5 px-4 py-3 dark:border-white/10"
      >
        <div className="flex-1">
          <SearchBar ref={searchRef} value={query} onChange={setQuery} />
        </div>
        <button
          type="button"
          title="Xoá lịch sử (giữ ghim)"
          onClick={async () => {
            await clearHistory(true);
            void reload();
          }}
          className="rounded-md p-1.5 text-neutral-500 hover:bg-black/10 dark:hover:bg-white/10"
        >
          <IconTrash className="h-4 w-4" />
        </button>
        <button
          type="button"
          title="Cài đặt"
          onClick={onOpenSettings}
          className="rounded-md p-1.5 text-neutral-500 hover:bg-black/10 dark:hover:bg-white/10"
        >
          <IconSettings className="h-4 w-4" />
        </button>
      </div>

      <div className="scroll-thin flex-1 overflow-y-auto p-4">
        {items.length === 0 ? (
          <EmptyState query={query} />
        ) : (
          <div className="flex flex-col gap-3">
            {pinned.length > 0 && (
              <section className="flex flex-col gap-2">
                <h2 className="px-1 text-[11px] font-semibold uppercase tracking-wide text-neutral-400">
                  Đã ghim
                </h2>
                {pinned.map((clip, i) => renderItem(clip, i))}
              </section>
            )}
            {rest.length > 0 && (
              <section className="flex flex-col gap-2">
                {!query && (
                  <h2 className="px-1 text-[11px] font-semibold uppercase tracking-wide text-neutral-400">
                    Gần đây
                  </h2>
                )}
                {rest.map((clip, i) => renderItem(clip, pinned.length + i))}
              </section>
            )}
          </div>
        )}
      </div>
    </div>
  );
}
