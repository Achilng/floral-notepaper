import { parseHotkey, type Hotkey } from "@tanstack/react-hotkeys";

const KEY_DISPLAY_NAMES: Record<string, string> = {
  Control: "Ctrl",
  Meta: "Meta",
  Backspace: "←",
  ArrowUp: "↑",
  ArrowDown: "↓",
  ArrowLeft: "←",
  ArrowRight: "→",
};

export function hotkeyToConfigString(hotkey: Hotkey): string {
  const parsed = parseHotkey(hotkey, "windows");
  const parts: string[] = [];
  if (parsed.ctrl) parts.push("Ctrl");
  if (parsed.alt) parts.push("Alt");
  if (parsed.shift) parts.push("Shift");
  if (parsed.meta) parts.push("Meta");
  parts.push(parsed.key);
  return parts.join("+");
}

export function isValidGlobalShortcut(hotkey: Hotkey): boolean {
  const parsed = parseHotkey(hotkey, "windows");
  return parsed.ctrl || parsed.alt || parsed.meta;
}

export function formatHeldKeys(keys: string[]): string {
  const modifierOrder = ["Control", "Alt", "Shift", "Meta"];
  const modifiers: string[] = [];
  const others: string[] = [];

  for (const key of keys) {
    if (modifierOrder.includes(key)) {
      modifiers.push(key);
    } else {
      others.push(key);
    }
  }

  modifiers.sort(
    (a, b) => modifierOrder.indexOf(a) - modifierOrder.indexOf(b),
  );

  const all = [...modifiers, ...others];
  return all.map((k) => KEY_DISPLAY_NAMES[k] ?? k).join(" + ");
}
