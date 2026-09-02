import { describe, expect, test, vi } from "vitest";
import { insertTextAtCursor } from "./useImagePaste";

function textarea(value: string, selectionStart: number, selectionEnd = selectionStart) {
  return {
    value,
    selectionStart,
    selectionEnd,
    focus: vi.fn(),
    setSelectionRange: vi.fn(),
  } as unknown as HTMLTextAreaElement;
}

describe("insertTextAtCursor", () => {
  test("inserts image Markdown and updates the controlled value", () => {
    const target = textarea("note", 4);
    const setContent = vi.fn();

    insertTextAtCursor(target, setContent, "![](images/note-1/photo.png)");

    const expected = "note\n![](images/note-1/photo.png)\n";
    expect(target.value).toBe(expected);
    expect(setContent).toHaveBeenCalledWith(expected);
    expect(target.setSelectionRange).toHaveBeenCalledWith(expected.length, expected.length);
  });

  test("replaces the selected text and preserves trailing content", () => {
    const target = textarea("before old after", 7, 10);
    const setContent = vi.fn();

    insertTextAtCursor(target, setContent, "![](images/note-1/photo.png)");

    expect(setContent).toHaveBeenCalledWith("before \n![](images/note-1/photo.png)\n after");
  });
});
