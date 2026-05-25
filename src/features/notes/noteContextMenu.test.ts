import { describe, expect, test } from "vitest";
import { getNoteContextMenuItems } from "./noteContextMenu";

describe("getNoteContextMenuItems", () => {
  test("includes export, move, and delete actions", () => {
    expect(getNoteContextMenuItems()).toEqual([
      { action: "export", label: "导出 Markdown" },
      { action: "move", label: "移动到分类…" },
      { action: "delete", label: "删除笔记", tone: "danger" },
    ]);
  });
});
