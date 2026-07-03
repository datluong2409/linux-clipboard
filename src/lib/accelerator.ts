// Translate a browser KeyboardEvent into a Tauri accelerator string and
// validate combos before we try to register them.

const MODIFIERS = ["Control", "Alt", "Shift", "Super"];

const CODE_MAP: Record<string, string> = {
  Space: "Space",
  Enter: "Enter",
  Tab: "Tab",
  Backspace: "Backspace",
  Delete: "Delete",
  Insert: "Insert",
  Home: "Home",
  End: "End",
  PageUp: "PageUp",
  PageDown: "PageDown",
  Minus: "-",
  Equal: "=",
  BracketLeft: "[",
  BracketRight: "]",
  Semicolon: ";",
  Quote: "'",
  Backquote: "`",
  Comma: ",",
  Period: ".",
  Slash: "/",
  Backslash: "\\",
};

function codeToKey(code: string): string | null {
  if (code.startsWith("Key")) return code.slice(3); // KeyV -> V
  if (code.startsWith("Digit")) return code.slice(5); // Digit1 -> 1
  if (code.startsWith("Numpad")) return code.slice(6);
  if (code.startsWith("Arrow")) return code.slice(5); // ArrowUp -> Up
  if (/^F\d{1,2}$/.test(code)) return code; // F5
  return CODE_MAP[code] ?? null;
}

/** Returns the accelerator string, or null while only modifiers are held. */
export function eventToAccelerator(e: KeyboardEvent): string | null {
  const parts: string[] = [];
  if (e.ctrlKey) parts.push("Control");
  if (e.altKey) parts.push("Alt");
  if (e.shiftKey) parts.push("Shift");
  if (e.metaKey) parts.push("Super");

  const key = codeToKey(e.code);
  if (!key) return null;
  parts.push(key);
  return parts.join("+");
}

/** Stable reason code the caller maps to a localized message (see i18n). */
export type AcceleratorError = "need_main_key" | "need_modifier";

export function isValidAccelerator(accel: string): {
  ok: boolean;
  reason?: AcceleratorError;
} {
  const parts = accel.split("+");
  const key = parts[parts.length - 1];
  const mods = parts.slice(0, -1);
  if (MODIFIERS.includes(key)) {
    return { ok: false, reason: "need_main_key" };
  }
  if (mods.length === 0) {
    return { ok: false, reason: "need_modifier" };
  }
  return { ok: true };
}

/** Pretty label for display, e.g. "Ctrl + Alt + V". */
export function prettyAccelerator(accel: string): string {
  return accel
    .split("+")
    .map((p) => (p === "Control" ? "Ctrl" : p))
    .join(" + ");
}
