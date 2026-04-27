import { describe, expect, it } from "vitest";
import {
  buildPreview,
  countNoteChars,
  filterNotes,
  getDisplayTitle,
} from "./noteUtils";
import type { NoteMetadata } from "./types";

const notes: NoteMetadata[] = [
  {
    id: "1",
    title: "读书笔记",
    fileName: "1.md",
    createdAt: "2026-04-28T01:00:00Z",
    updatedAt: "2026-04-28T01:00:00Z",
    wordCount: 20,
    preview: "关于月亮与六便士",
  },
  {
    id: "2",
    title: "",
    fileName: "2.md",
    createdAt: "2026-04-28T02:00:00Z",
    updatedAt: "2026-04-28T02:00:00Z",
    wordCount: 12,
    preview: "周末采购清单",
  },
];

describe("note utilities", () => {
  it("uses title, preview, then untitled fallback for display title", () => {
    expect(getDisplayTitle(notes[0])).toBe("读书笔记");
    expect(getDisplayTitle(notes[1])).toBe("周末采购清单");
    expect(getDisplayTitle({ ...notes[1], preview: "" })).toBe("无标题笔记");
  });

  it("builds compact previews and counts non-whitespace characters", () => {
    const content = "# 标题\n\n正文 第一行\n第二行";

    expect(buildPreview(content)).toBe("# 标题 正文 第一行 第二行");
    expect(countNoteChars(content)).toBe(11);
  });

  it("filters notes by title, preview, or file name without case sensitivity", () => {
    expect(filterNotes(notes, "读书").map((note) => note.id)).toEqual(["1"]);
    expect(filterNotes(notes, "采购").map((note) => note.id)).toEqual(["2"]);
    expect(filterNotes(notes, "2.MD").map((note) => note.id)).toEqual(["2"]);
    expect(filterNotes(notes, "   ").map((note) => note.id)).toEqual(["1", "2"]);
  });
});
