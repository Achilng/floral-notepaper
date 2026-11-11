import { describe, expect, test } from "vitest";
import { getTileContextMenuItems } from "./tileContextMenu";

describe("getTileContextMenuItems", () => {
  test("keeps the base actions and closes with a danger tone", () => {
    const items = getTileContextMenuItems().filter(
      (item): item is Exclude<typeof item, { separator: true }> => "action" in item,
    );

    expect(items.map((item) => item.action)).toEqual([
      "copy",
      "save",
      "switchToPad",
      "layerTop",
      "layerNormal",
      "layerBottom",
      "close",
    ]);
    expect(items[items.length - 1]?.tone).toBe("danger");
  });

  test("marks only the current layer as checked", () => {
    for (const layer of ["top", "normal", "bottom"] as const) {
      const items = getTileContextMenuItems(undefined, layer).filter(
        (item): item is Exclude<typeof item, { separator: true }> => "action" in item,
      );
      const layerItems = items.filter((item) => item.action.startsWith("layer"));

      expect(layerItems.map((item) => item.label.startsWith("✓ "))).toEqual(
        layerItems.map((item) => item.action === `layer${layer[0].toUpperCase()}${layer.slice(1)}`),
      );
    }
  });

  test("separates the layer group and the close action", () => {
    const entries = getTileContextMenuItems();
    expect(entries.filter((entry) => "separator" in entry)).toHaveLength(2);
  });
});
