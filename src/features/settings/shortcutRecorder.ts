import { parseHotkey, type Hotkey } from "@tanstack/react-hotkeys";

export type ShortcutPlatform = "macos" | "windows";

const KEY_DISPLAY_NAMES: Record<string, string> = {
  Control: "Ctrl",
  Meta: "Meta",
  Backspace: "←",
  ArrowUp: "↑",
  ArrowDown: "↓",
  ArrowLeft: "←",
  ArrowRight: "→",
};

const MAC_KEY_DISPLAY_NAMES: Record<string, string> = {
  ...KEY_DISPLAY_NAMES,
  Control: "Ctrl",
  Alt: "Option",
  Meta: "Command",
};

export function shortcutPlatform(): ShortcutPlatform {
  if (typeof navigator !== "undefined" && /Mac|iPhone|iPad/.test(navigator.platform)) {
    return "macos";
  }

  return "windows";
}

export function hotkeyToConfigString(
  hotkey: Hotkey,
  platform: ShortcutPlatform = "windows",
): string {
  const parsed = parseHotkey(hotkey, "windows");
  const parts: string[] = [];
  if (platform === "macos") {
    if (parsed.meta) parts.push("Command");
    if (parsed.alt) parts.push("Option");
    if (parsed.ctrl) parts.push("Ctrl");
    if (parsed.shift) parts.push("Shift");
  } else {
    if (parsed.ctrl) parts.push("Ctrl");
    if (parsed.alt) parts.push("Alt");
    if (parsed.shift) parts.push("Shift");
    if (parsed.meta) parts.push("Meta");
  }
  parts.push(parsed.key);
  return parts.join("+");
}

export function isValidGlobalShortcut(hotkey: Hotkey): boolean {
  const parsed = parseHotkey(hotkey, "windows");
  return parsed.ctrl || parsed.alt || parsed.meta;
}

export function formatHeldKeys(
  keys: string[],
  platform: ShortcutPlatform = "windows",
): string {
  const modifierOrder =
    platform === "macos"
      ? ["Meta", "Alt", "Control", "Shift"]
      : ["Control", "Alt", "Shift", "Meta"];
  const modifiers: string[] = [];
  const others: string[] = [];

  for (const key of keys) {
    if (modifierOrder.includes(key)) {
      modifiers.push(key);
    } else {
      others.push(key);
    }
  }

  modifiers.sort((a, b) => modifierOrder.indexOf(a) - modifierOrder.indexOf(b));

  const all = [...modifiers, ...others];
  const displayNames =
    platform === "macos" ? MAC_KEY_DISPLAY_NAMES : KEY_DISPLAY_NAMES;
  return all.map((k) => displayNames[k] ?? k).join(" + ");
}
