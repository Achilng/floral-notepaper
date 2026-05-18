import { describe, expect, test } from "vitest";
import { noteContextMenuItems } from "./noteContextMenu";

describe("noteContextMenuItems", () => {
  test("includes export, move, and delete actions with translation keys", () => {
    expect(noteContextMenuItems).toEqual([
      { action: "export", label: "main.contextMenu.export" },
      { action: "move", label: "main.contextMenu.moveToCategory" },
      { action: "delete", label: "main.contextMenu.deleteNote", tone: "danger" },
    ]);
  });
});
