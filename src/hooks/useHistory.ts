import { useCallback, useEffect, useState } from "react";
import { getHistory, onEvent, searchHistory } from "../lib/ipc";
import type { Clip } from "../types";

/** Loads history (or search results), and re-loads on `history-updated` and
 *  whenever `refreshKey` changes (i.e. each time the panel is shown). */
export function useHistory(query: string, refreshKey: number) {
  const [items, setItems] = useState<Clip[]>([]);

  const load = useCallback(async () => {
    const res = query.trim() ? await searchHistory(query) : await getHistory();
    setItems(res);
  }, [query]);

  useEffect(() => {
    void load();
  }, [load, refreshKey]);

  useEffect(() => {
    const un = onEvent("history-updated", () => void load());
    return () => {
      void un.then((u) => u());
    };
  }, [load]);

  return { items, reload: load };
}
