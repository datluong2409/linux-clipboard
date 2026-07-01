export function Toast({ message }: { message: string }) {
  return (
    <div className="pointer-events-none absolute inset-x-0 bottom-3 flex justify-center px-4">
      <div className="rounded-md bg-neutral-900/90 px-3 py-1.5 text-center text-xs text-white shadow-lg">
        {message}
      </div>
    </div>
  );
}
