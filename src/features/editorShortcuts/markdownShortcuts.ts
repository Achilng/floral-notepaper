import type { EditorShortcuts } from "../settings/types";

export type MarkdownShortcutCommand =
  | "paragraph"
  | "heading1"
  | "heading2"
  | "heading3"
  | "heading4"
  | "heading5"
  | "heading6"
  | "bold"
  | "italic"
  | "strike"
  | "quote"
  | "unorderedList"
  | "orderedList"
  | "codeBlock";

export interface MarkdownShortcutDescriptor {
  command: MarkdownShortcutCommand;
  labelKey: string;
  defaultLabel: string;
}

export interface TextSelection {
  start: number;
  end: number;
}

export interface MarkdownTransformResult {
  value: string;
  selectionStart: number;
  selectionEnd: number;
}

export const DEFAULT_EDITOR_SHORTCUTS: EditorShortcuts = {
  // 备注：这些快捷键只在编辑器 textarea 获得焦点时生效，不会注册为系统全局快捷键。
  paragraph: "Ctrl+0",
  heading1: "Ctrl+1",
  heading2: "Ctrl+2",
  heading3: "Ctrl+3",
  heading4: "Ctrl+4",
  heading5: "Ctrl+5",
  heading6: "Ctrl+6",
  bold: "Ctrl+B",
  italic: "Ctrl+I",
  strike: "Ctrl+Shift+X",
  quote: "Ctrl+Shift+Q",
  unorderedList: "Ctrl+Shift+L",
  orderedList: "Ctrl+Shift+O",
  codeBlock: "Ctrl+Shift+K",
};

export const MARKDOWN_SHORTCUTS: MarkdownShortcutDescriptor[] = [
  {
    command: "paragraph",
    labelKey: "settings.editorShortcuts.paragraph",
    defaultLabel: "普通段落",
  },
  { command: "heading1", labelKey: "settings.editorShortcuts.heading1", defaultLabel: "一级标题" },
  { command: "heading2", labelKey: "settings.editorShortcuts.heading2", defaultLabel: "二级标题" },
  { command: "heading3", labelKey: "settings.editorShortcuts.heading3", defaultLabel: "三级标题" },
  { command: "heading4", labelKey: "settings.editorShortcuts.heading4", defaultLabel: "四级标题" },
  { command: "heading5", labelKey: "settings.editorShortcuts.heading5", defaultLabel: "五级标题" },
  { command: "heading6", labelKey: "settings.editorShortcuts.heading6", defaultLabel: "六级标题" },
  { command: "bold", labelKey: "settings.editorShortcuts.bold", defaultLabel: "粗体" },
  { command: "italic", labelKey: "settings.editorShortcuts.italic", defaultLabel: "斜体" },
  { command: "strike", labelKey: "settings.editorShortcuts.strike", defaultLabel: "删除线" },
  { command: "quote", labelKey: "settings.editorShortcuts.quote", defaultLabel: "引用" },
  {
    command: "unorderedList",
    labelKey: "settings.editorShortcuts.unorderedList",
    defaultLabel: "无序列表",
  },
  {
    command: "orderedList",
    labelKey: "settings.editorShortcuts.orderedList",
    defaultLabel: "有序列表",
  },
  { command: "codeBlock", labelKey: "settings.editorShortcuts.codeBlock", defaultLabel: "代码块" },
];

export function normalizeEditorShortcuts(shortcuts?: Partial<EditorShortcuts>): EditorShortcuts {
  return { ...DEFAULT_EDITOR_SHORTCUTS, ...shortcuts };
}

export function editorShortcutConflicts(shortcuts: EditorShortcuts): Set<MarkdownShortcutCommand> {
  const seen = new Map<string, MarkdownShortcutCommand>();
  const conflicts = new Set<MarkdownShortcutCommand>();
  for (const { command } of MARKDOWN_SHORTCUTS) {
    const shortcut = shortcuts[command].trim().toLowerCase();
    if (!shortcut) continue;
    const previous = seen.get(shortcut);
    if (previous) {
      conflicts.add(previous);
      conflicts.add(command);
    } else {
      seen.set(shortcut, command);
    }
  }
  return conflicts;
}

export function commandForKeyboardEvent(
  event: Pick<KeyboardEvent, "key" | "ctrlKey" | "altKey" | "shiftKey" | "metaKey">,
  shortcuts: EditorShortcuts,
): MarkdownShortcutCommand | null {
  for (const { command } of MARKDOWN_SHORTCUTS) {
    if (matchesShortcut(event, shortcuts[command])) {
      return command;
    }
  }
  return null;
}

function matchesShortcut(
  event: Pick<KeyboardEvent, "key" | "ctrlKey" | "altKey" | "shiftKey" | "metaKey">,
  shortcut: string,
): boolean {
  const parts = shortcut
    .split("+")
    .map((part) => part.trim())
    .filter(Boolean);
  if (parts.length === 0) return false;

  const key = parts[parts.length - 1]?.toLowerCase();
  const modifiers = new Set(parts.slice(0, -1).map((part) => part.toLowerCase()));
  const wantsCtrl = modifiers.has("ctrl") || modifiers.has("control");
  const wantsAlt = modifiers.has("alt") || modifiers.has("option");
  const wantsShift = modifiers.has("shift");
  const wantsMeta = modifiers.has("meta") || modifiers.has("cmd") || modifiers.has("command");

  return (
    event.ctrlKey === wantsCtrl &&
    event.altKey === wantsAlt &&
    event.shiftKey === wantsShift &&
    event.metaKey === wantsMeta &&
    normalizeEventKey(event.key) === key
  );
}

function normalizeEventKey(key: string): string {
  if (key === " ") return "space";
  if (key.length === 1) return key.toLowerCase();
  return key.toLowerCase();
}

export function applyMarkdownShortcut(
  value: string,
  selection: TextSelection,
  command: MarkdownShortcutCommand,
): MarkdownTransformResult {
  if (command.startsWith("heading")) {
    const level = Number(command.replace("heading", ""));
    return transformLines(value, selection, (lines) => transformHeadingLines(lines, level));
  }
  if (command === "paragraph") {
    return transformLines(value, selection, removeBlockPrefixes);
  }
  if (command === "quote") {
    return transformLines(value, selection, toggleLinePrefix(/^>\s?/, "> "));
  }
  if (command === "unorderedList") {
    return transformLines(value, selection, toggleLinePrefix(/^[-*+]\s+/, "- "));
  }
  if (command === "orderedList") {
    return transformLines(value, selection, toggleOrderedList);
  }
  if (command === "codeBlock") {
    return transformCodeBlock(value, selection);
  }
  if (command === "bold") {
    return toggleInline(value, selection, "**", "粗体文本");
  }
  if (command === "italic") {
    return toggleInline(value, selection, "*", "斜体文本");
  }
  return toggleInline(value, selection, "~~", "删除线文本");
}

export function runMarkdownShortcutOnTextarea(
  textarea: HTMLTextAreaElement,
  command: MarkdownShortcutCommand,
  setContent: (value: string) => void,
  markDirty: () => void,
): void {
  const result = applyMarkdownShortcut(
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

function toggleInline(
  value: string,
  selection: TextSelection,
  marker: string,
  fallback: string,
): MarkdownTransformResult {
  const selected = value.slice(selection.start, selection.end) || fallback;
  const beforeMarkerStart = selection.start - marker.length;
  const afterMarkerEnd = selection.end + marker.length;
  const hasMarkers =
    beforeMarkerStart >= 0 &&
    value.slice(beforeMarkerStart, selection.start) === marker &&
    value.slice(selection.end, afterMarkerEnd) === marker;

  if (hasMarkers) {
    const next =
      value.slice(0, beforeMarkerStart) +
      value.slice(selection.start, selection.end) +
      value.slice(afterMarkerEnd);
    return {
      value: next,
      selectionStart: beforeMarkerStart,
      selectionEnd: beforeMarkerStart + selection.end - selection.start,
    };
  }

  const wrapped = `${marker}${selected}${marker}`;
  return {
    value: value.slice(0, selection.start) + wrapped + value.slice(selection.end),
    selectionStart: selection.start + marker.length,
    selectionEnd: selection.start + marker.length + selected.length,
  };
}

function transformLines(
  value: string,
  selection: TextSelection,
  transform: (lines: string[]) => string[],
): MarkdownTransformResult {
  const lineStart = value.lastIndexOf("\n", selection.start - 1) + 1;
  const endIndex = selection.end > selection.start ? selection.end - 1 : selection.end;
  const nextLineBreak = value.indexOf("\n", endIndex);
  const lineEnd = nextLineBreak === -1 ? value.length : nextLineBreak;
  const originalBlock = value.slice(lineStart, lineEnd);
  const transformedBlock = transform(originalBlock.split("\n")).join("\n");

  // 备注：块级快捷键作用于当前行或选区覆盖的整行，和思源的块转换思路保持一致。
  return {
    value: value.slice(0, lineStart) + transformedBlock + value.slice(lineEnd),
    selectionStart: lineStart,
    selectionEnd: lineStart + transformedBlock.length,
  };
}

function transformHeadingLines(lines: string[], level: number): string[] {
  const marker = `${"#".repeat(level)} `;
  const nonEmpty = lines.filter((line) => line.trim().length > 0);
  const allSameHeading =
    nonEmpty.length > 0 && nonEmpty.every((line) => line.match(/^(#{1,6})\s+/)?.[0] === marker);

  // 备注：再次按同级标题快捷键时取消标题，恢复为普通段落。
  if (allSameHeading) {
    return lines.map((line) => line.replace(/^#{1,6}\s+/, ""));
  }

  return lines.map((line) => marker + line.replace(/^#{1,6}\s+/, ""));
}

function removeBlockPrefixes(lines: string[]): string[] {
  return lines.map((line) =>
    line
      .replace(/^#{1,6}\s+/, "")
      .replace(/^>\s?/, "")
      .replace(/^[-*+]\s+/, "")
      .replace(/^\d+\.\s+/, ""),
  );
}

function toggleLinePrefix(removePattern: RegExp, prefix: string): (lines: string[]) => string[] {
  return (lines) => {
    const nonEmpty = lines.filter((line) => line.trim().length > 0);
    const shouldRemove = nonEmpty.length > 0 && nonEmpty.every((line) => removePattern.test(line));
    return lines.map((line) =>
      shouldRemove ? line.replace(removePattern, "") : prefix + line.replace(removePattern, ""),
    );
  };
}

function toggleOrderedList(lines: string[]): string[] {
  const nonEmpty = lines.filter((line) => line.trim().length > 0);
  const shouldRemove = nonEmpty.length > 0 && nonEmpty.every((line) => /^\d+\.\s+/.test(line));
  return lines.map((line, index) =>
    shouldRemove ? line.replace(/^\d+\.\s+/, "") : `${index + 1}. ${line.replace(/^\d+\.\s+/, "")}`,
  );
}

function transformCodeBlock(value: string, selection: TextSelection): MarkdownTransformResult {
  const selected = value.slice(selection.start, selection.end);
  if (selected) {
    // 备注：有选区时包裹为 fenced code block，默认语言使用 text，避免误判语言。
    const wrapped = `\`\`\`text\n${selected}\n\`\`\``;
    return {
      value: value.slice(0, selection.start) + wrapped + value.slice(selection.end),
      selectionStart: selection.start + "```text\n".length,
      selectionEnd: selection.start + "```text\n".length + selected.length,
    };
  }

  // 备注：无选区时插入空代码块，并把光标放在中间空行，方便直接输入代码。
  const block = "```text\n\n```";
  return {
    value: value.slice(0, selection.start) + block + value.slice(selection.end),
    selectionStart: selection.start + "```text\n".length,
    selectionEnd: selection.start + "```text\n".length,
  };
}
