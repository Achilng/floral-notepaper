import { describe, expect, test } from "vitest";
import { noteContextMenuItems } from "./noteContextMenu";

describe("noteContextMenuItems", () => {
  test("includes pin, export, move, and delete actions", () => {
    expect(noteContextMenuItems).toMatchObject([
      { action: "pin" },
      { action: "export", label: "导出 Markdown" },
      { action: "move", label: "移动到分类…" },
      { action: "delete", label: "删除笔记", tone: "danger" },
    ]);
    const pinItem = noteContextMenuItems[0];
    expect(typeof pinItem.label).toBe("function");
    if (typeof pinItem.label === "function") {
      expect(pinItem.label(false)).toBe("置顶笔记");
      expect(pinItem.label(true)).toBe("取消置顶");
    }
  });
});
