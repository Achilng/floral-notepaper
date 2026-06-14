import { describe, expect, test } from "vitest";
import {
  DEFAULT_EDITOR_SHORTCUTS,
  applyEditorShortcut,
  commandForKeyboardEvent,
  editorShortcutBindings,
  normalizeEditorShortcuts,
} from "./markdownShortcuts";

describe("markdownShortcuts", () => {
  test("normalizes missing editor shortcut config", () => {
    expect(normalizeEditorShortcuts({ heading1: "Alt+1" })).toEqual({
      ...DEFAULT_EDITOR_SHORTCUTS,
      heading1: "Alt+1",
    });
  });

  test("exposes non-empty shortcut bindings in command order", () => {
    const bindings = editorShortcutBindings({
      ...DEFAULT_EDITOR_SHORTCUTS,
      paragraph: "",
    });

    expect(bindings[0]).toEqual({ command: "heading1", shortcut: "Ctrl+1" });
  });

  test("matches configured heading shortcuts", () => {
    expect(
      commandForKeyboardEvent(
        { key: "2", ctrlKey: true, altKey: false, shiftKey: false, metaKey: false },
        DEFAULT_EDITOR_SHORTCUTS,
      ),
    ).toBe("heading2");
  });

  test("sets and toggles heading levels", () => {
    const heading = applyEditorShortcut("Title", { start: 2, end: 2 }, "heading2");
    expect(heading.value).toBe("## Title");

    const paragraph = applyEditorShortcut(heading.value, { start: 3, end: 3 }, "heading2");
    expect(paragraph.value).toBe("Title");
  });

  test("converts selected heading lines back to paragraphs", () => {
    const result = applyEditorShortcut("## A\n### B", { start: 0, end: 10 }, "paragraph");
    expect(result.value).toBe("A\nB");
  });
});
