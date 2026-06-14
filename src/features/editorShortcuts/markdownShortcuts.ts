import type { EditorShortcuts } from "../settings/types";

export type EditorShortcutCommand =
  | "paragraph"
  | "heading1"
  | "heading2"
  | "heading3"
  | "heading4"
  | "heading5"
  | "heading6";

interface ShortcutBinding {
  command: EditorShortcutCommand;
  shortcut: string;
}

export interface TextSelection {
  start: number;
  end: number;
}

export interface TextTransformResult {
  value: string;
  selectionStart: number;
  selectionEnd: number;
}

export const DEFAULT_EDITOR_SHORTCUTS: EditorShortcuts = {
  paragraph: "Ctrl+0",
  heading1: "Ctrl+1",
  heading2: "Ctrl+2",
  heading3: "Ctrl+3",
  heading4: "Ctrl+4",
  heading5: "Ctrl+5",
  heading6: "Ctrl+6",
};

const EDITOR_SHORTCUT_COMMANDS: EditorShortcutCommand[] = [
  // 备注：这里先保留编辑器局部快捷键的命令映射入口，后续自定义 UI 可复用同一分发链路。
  "paragraph",
  "heading1",
  "heading2",
  "heading3",
  "heading4",
  "heading5",
  "heading6",
];

export function normalizeEditorShortcuts(shortcuts?: Partial<EditorShortcuts>): EditorShortcuts {
  return { ...DEFAULT_EDITOR_SHORTCUTS, ...shortcuts };
}

export function editorShortcutBindings(shortcuts: EditorShortcuts): ShortcutBinding[] {
  return EDITOR_SHORTCUT_COMMANDS.map((command) => ({
    command,
    shortcut: shortcuts[command],
  })).filter(({ shortcut }) => shortcut.trim().length > 0);
}

export function commandForKeyboardEvent(
  event: Pick<KeyboardEvent, "key" | "ctrlKey" | "altKey" | "shiftKey" | "metaKey">,
  shortcuts: EditorShortcuts,
): EditorShortcutCommand | null {
  const binding = editorShortcutBindings(shortcuts).find(({ shortcut }) =>
    matchesShortcut(event, shortcut),
  );
  return binding?.command ?? null;
}

export function applyEditorShortcut(
  value: string,
  selection: TextSelection,
  command: EditorShortcutCommand,
): TextTransformResult {
  if (command === "paragraph") {
    return transformSelectedLines(value, selection, (lines) =>
      lines.map((line) => line.replace(/^#{1,6}\s+/, "")),
    );
  }

  const level = Number(command.replace("heading", ""));
  return transformSelectedLines(value, selection, (lines) => toggleHeading(lines, level));
}

export function runEditorShortcutOnTextarea(
  textarea: HTMLTextAreaElement,
  command: EditorShortcutCommand,
  setContent: (value: string) => void,
  markDirty: () => void,
): void {
  const result = applyEditorShortcut(
    textarea.value,
    { start: textarea.selectionStart, end: textarea.selectionEnd },
    command,
  );

  textarea.focus();
  textarea.setSelectionRange(0, textarea.value.length);
  document.execCommand("insertText", false, result.value);
  setContent(result.value);
  markDirty();
  requestAnimationFrame(() => {
    textarea.setSelectionRange(result.selectionStart, result.selectionEnd);
  });
}

function transformSelectedLines(
  value: string,
  selection: TextSelection,
  transform: (lines: string[]) => string[],
): TextTransformResult {
  const start = value.lastIndexOf("\n", selection.start - 1) + 1;
  const endIndex = selection.end > selection.start ? selection.end - 1 : selection.end;
  const nextLineBreak = value.indexOf("\n", endIndex);
  const end = nextLineBreak === -1 ? value.length : nextLineBreak;
  const nextBlock = transform(value.slice(start, end).split("\n")).join("\n");

  return {
    value: value.slice(0, start) + nextBlock + value.slice(end),
    selectionStart: start,
    selectionEnd: start + nextBlock.length,
  };
}

function toggleHeading(lines: string[], level: number): string[] {
  const marker = `${"#".repeat(level)} `;
  const nonEmpty = lines.filter((line) => line.trim().length > 0);
  const alreadySameLevel =
    nonEmpty.length > 0 && nonEmpty.every((line) => line.match(/^(#{1,6})\s+/)?.[0] === marker);

  return lines.map((line) =>
    alreadySameLevel ? line.replace(/^#{1,6}\s+/, "") : marker + line.replace(/^#{1,6}\s+/, ""),
  );
}

function matchesShortcut(
  event: Pick<KeyboardEvent, "key" | "ctrlKey" | "altKey" | "shiftKey" | "metaKey">,
  shortcut: string,
): boolean {
  const parts = shortcut
    .split("+")
    .map((part) => part.trim().toLowerCase())
    .filter(Boolean);
  const key = parts[parts.length - 1];
  const modifiers = new Set(parts.slice(0, -1));

  return (
    Boolean(key) &&
    event.ctrlKey === (modifiers.has("ctrl") || modifiers.has("control")) &&
    event.altKey === (modifiers.has("alt") || modifiers.has("option")) &&
    event.shiftKey === modifiers.has("shift") &&
    event.metaKey === (modifiers.has("meta") || modifiers.has("cmd") || modifiers.has("command")) &&
    normalizeKey(event.key) === key
  );
}

function normalizeKey(key: string): string {
  return key === " " ? "space" : key.toLowerCase();
}
