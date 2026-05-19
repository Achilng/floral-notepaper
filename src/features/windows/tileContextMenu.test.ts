import { describe, expect, test } from "vitest";
import { getTileContextMenuItems } from "./tileContextMenu";

describe("getTileContextMenuItems", () => {
  test("offers useful tile actions", () => {
    expect(getTileContextMenuItems()).toEqual([
      { action: "copy", label: "复制" },
      { action: "save", label: "保存" },
      { action: "switchToPad", label: "转为小窗" },
      { action: "close", label: "取消钉屏", tone: "danger" },
    ]);
  });
});
