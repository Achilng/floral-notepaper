import { describe, expect, test } from "vitest";
import { noteContextMenuItems } from "./noteContextMenu";

describe("noteContextMenuItems", () => {
  test("keeps note list actions focused on export and delete", () => {
    expect(noteContextMenuItems).toEqual([
      { action: "export", label: "导出 Markdown" },
      { action: "delete", label: "删除笔记", tone: "danger" },
    ]);
  });
});
