import { IconClipboard } from "./Icons";

export function EmptyState({ query }: { query: string }) {
  return (
    <div className="flex h-full flex-col items-center justify-center gap-2 px-6 text-center text-neutral-500 dark:text-neutral-400">
      <IconClipboard className="h-8 w-8 opacity-40" />
      <p className="text-sm">
        {query ? "Không tìm thấy kết quả" : "Chưa có gì trong lịch sử clipboard"}
      </p>
      {!query && (
        <p className="text-xs opacity-70">Copy (Ctrl+C) ở bất kỳ đâu để bắt đầu.</p>
      )}
    </div>
  );
}
