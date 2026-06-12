import { describe, expect, test } from "vitest";
import {
  DEFAULT_EDITOR_SHORTCUTS,
  applyMarkdownShortcut,
  commandForKeyboardEvent,
  editorShortcutConflicts,
} from "./markdownShortcuts";

describe("markdownShortcuts", () => {
  test("sets and toggles heading levels", () => {
    const heading = applyMarkdownShortcut("Title", { start: 2, end: 2 }, "heading2");
    expect(heading.value).toBe("## Title");

    const paragraph = applyMarkdownShortcut(heading.value, { start: 3, end: 3 }, "heading2");
    expect(paragraph.value).toBe("Title");
  });

  test("turns selected lines into ordered and unordered lists", () => {
    const unordered = applyMarkdownShortcut("a\nb", { start: 0, end: 3 }, "unorderedList");
    expect(unordered.value).toBe("- a\n- b");

    const ordered = applyMarkdownShortcut("a\nb", { start: 0, end: 3 }, "orderedList");
    expect(ordered.value).toBe("1. a\n2. b");
  });

  test("toggles quotes and converts block prefixes back to paragraphs", () => {
    const quote = applyMarkdownShortcut("note", { start: 0, end: 4 }, "quote");
    expect(quote.value).toBe("> note");

    const paragraph = applyMarkdownShortcut("## Title\n- item", { start: 0, end: 15 }, "paragraph");
    expect(paragraph.value).toBe("Title\nitem");
  });

  test("toggles inline marks", () => {
    const bold = applyMarkdownShortcut("hello", { start: 0, end: 5 }, "bold");
    expect(bold.value).toBe("**hello**");

    const plain = applyMarkdownShortcut(bold.value, { start: 2, end: 7 }, "bold");
    expect(plain.value).toBe("hello");
  });

  test("inserts code blocks with and without selection", () => {
    const selected = applyMarkdownShortcut("print(1)", { start: 0, end: 8 }, "codeBlock");
    expect(selected.value).toBe("```text\nprint(1)\n```");

    const empty = applyMarkdownShortcut("", { start: 0, end: 0 }, "codeBlock");
    expect(empty.value).toBe("```text\n\n```");
    expect(empty.selectionStart).toBe("```text\n".length);
  });

  test("matches configured shortcuts and detects duplicates", () => {
    expect(
      commandForKeyboardEvent(
        { key: "1", ctrlKey: true, altKey: false, shiftKey: false, metaKey: false },
        DEFAULT_EDITOR_SHORTCUTS,
      ),
    ).toBe("heading1");

    const conflicts = editorShortcutConflicts({
      ...DEFAULT_EDITOR_SHORTCUTS,
      bold: "Ctrl+1",
    });
    expect(conflicts.has("heading1")).toBe(true);
    expect(conflicts.has("bold")).toBe(true);
  });
});
