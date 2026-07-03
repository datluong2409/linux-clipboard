import { useI18n } from "../lib/i18n";
import { IconClipboard } from "./Icons";

export function EmptyState({ query }: { query: string }) {
  const { t } = useI18n();
  return (
    <div className="flex h-full flex-col items-center justify-center gap-2 px-6 text-center text-neutral-500 dark:text-neutral-400">
      <IconClipboard className="h-8 w-8 opacity-40" />
      <p className="text-sm">{query ? t("emptyNoResults") : t("emptyNothing")}</p>
      {!query && <p className="text-xs opacity-70">{t("emptyHint")}</p>}
    </div>
  );
}
