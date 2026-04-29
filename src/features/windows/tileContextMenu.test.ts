import { describe, expect, test } from "vitest";
import { tileContextMenuItems } from "./tileContextMenu";

describe("tileContextMenuItems", () => {
  test("offers useful tile actions", () => {
    expect(tileContextMenuItems).toEqual([
      { action: "copy", label: "复制" },
      { action: "save", label: "保存" },
      { action: "switchToPad", label: "转为小窗" },
      { action: "close", label: "关闭", tone: "danger" },
    ]);
  });
});
