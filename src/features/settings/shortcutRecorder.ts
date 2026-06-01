export type ShortcutPlatform = "mac" | "windows";

interface ParsedShortcut {
  ctrl: boolean;
  alt: boolean;
  shift: boolean;
  meta: boolean;
  key: string;
}

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
    return "mac";
  }

  return "windows";
}

function parseShortcutString(
  shortcut: string,
  platform: ShortcutPlatform = "windows",
): ParsedShortcut {
  const parts = shortcut.split("+");
  const result: ParsedShortcut = { ctrl: false, alt: false, shift: false, meta: false, key: "" };
  for (const part of parts) {
    switch (part) {
      case "Control":
      case "Ctrl":
        result.ctrl = true;
        break;
      case "Alt":
      case "Option":
        result.alt = true;
        break;
      case "Shift":
        result.shift = true;
        break;
      case "Meta":
      case "Command":
        result.meta = true;
        break;
      case "Mod":
        if (platform === "mac") result.meta = true;
        else result.ctrl = true;
        break;
      default:
        result.key = part;
        break;
    }
  }
  return result;
}

export function hotkeyToConfigString(
  shortcut: string,
  platform: ShortcutPlatform = "windows",
): string {
  const parsed = parseShortcutString(shortcut, platform);
  const parts: string[] = [];
  if (platform === "mac") {
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

export function isValidGlobalShortcut(shortcut: string): boolean {
  const parsed = parseShortcutString(shortcut, "windows");
  return parsed.ctrl || parsed.alt || parsed.meta;
}

export function formatHeldKeys(keys: string[], platform: ShortcutPlatform = "windows"): string {
  const modifierOrder =
    platform === "mac" ? ["Meta", "Alt", "Control", "Shift"] : ["Control", "Alt", "Shift", "Meta"];
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
  const displayNames = platform === "mac" ? MAC_KEY_DISPLAY_NAMES : KEY_DISPLAY_NAMES;
  return all.map((k) => displayNames[k] ?? k).join(" + ");
}
