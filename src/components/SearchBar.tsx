import { forwardRef } from "react";
import { IconSearch } from "./Icons";

interface Props {
  value: string;
  onChange: (v: string) => void;
}

export const SearchBar = forwardRef<HTMLInputElement, Props>(function SearchBar(
  { value, onChange },
  ref,
) {
  return (
    <div className="relative">
      <IconSearch className="pointer-events-none absolute left-2.5 top-1/2 h-4 w-4 -translate-y-1/2 text-neutral-400" />
      <input
        ref={ref}
        value={value}
        onChange={(e) => onChange(e.target.value)}
        placeholder="Tìm kiếm..."
        spellCheck={false}
        className="w-full rounded-md border border-black/10 bg-white/70 py-1.5 pl-8 pr-2 text-sm text-neutral-800 outline-none placeholder:text-neutral-400 focus:border-[var(--color-accent)] dark:border-white/10 dark:bg-white/10 dark:text-neutral-100"
      />
    </div>
  );
});
