import { describe, expect, it } from "vitest";
import { toggleHighlightSelection } from "./highlightSelection";

describe("toggleHighlightSelection", () => {
  it("wraps the selection with highlight markers", () => {
    const edit = toggleHighlightSelection("a b c", 2, 3, "高亮文本");

    expect(edit.result).toBe("a ==b== c");
    expect(edit.cursorStart).toBe(4);
    expect(edit.cursorEnd).toBe(5);
  });

  it("keeps the selection on the content when wrapping", () => {
    const value = "重要内容";
    const edit = toggleHighlightSelection(value, 0, value.length, "高亮文本");

    expect(edit.result).toBe("==重要内容==");
    expect(edit.cursorStart).toBe(2);
    expect(edit.cursorEnd).toBe(6);
  });

  it("inserts the sample text when there is no selection", () => {
    const edit = toggleHighlightSelection("", 0, 0, "高亮文本");

    expect(edit.result).toBe("==高亮文本==");
    expect(edit.cursorStart).toBe(2);
    expect(edit.cursorEnd).toBe(6);
  });

  it("removes markers surrounding the selection", () => {
    const edit = toggleHighlightSelection("a ==b== c", 4, 5, "高亮文本");

    expect(edit.result).toBe("a b c");
    expect(edit.cursorStart).toBe(2);
    expect(edit.cursorEnd).toBe(3);
  });

  it("removes markers when the selection includes them", () => {
    const edit = toggleHighlightSelection("a ==b== c", 2, 7, "高亮文本");

    expect(edit.result).toBe("a b c");
    expect(edit.cursorStart).toBe(2);
    expect(edit.cursorEnd).toBe(3);
  });

  it("wraps multi-line selections as a single span", () => {
    const edit = toggleHighlightSelection("l1\nl2", 0, 5, "高亮文本");

    expect(edit.result).toBe("==l1\nl2==");
    expect(edit.cursorStart).toBe(2);
    expect(edit.cursorEnd).toBe(7);
  });

  it("keeps spaced markers unresolved but wraps a fresh selection", () => {
    const edit = toggleHighlightSelection("a == b == c", 5, 6, "高亮文本");

    expect(edit.result).toBe("a == ==b== == c");
  });
});
