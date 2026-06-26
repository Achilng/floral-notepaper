import { describe, expect, test } from "vitest";
import {
  FONT_ZOOM_MAX,
  FONT_ZOOM_MIN,
  getFontZoomDirection,
  nextFontSize,
} from "./fontZoom";

const keyEvent = (
  overrides: Partial<Parameters<typeof getFontZoomDirection>[0]>,
): Parameters<typeof getFontZoomDirection>[0] => ({
  altKey: false,
  ctrlKey: true,
  metaKey: false,
  key: "",
  code: "",
  ...overrides,
});

describe("fontZoom", () => {
  test("detects Obsidian-style zoom-in shortcuts", () => {
    expect(getFontZoomDirection(keyEvent({ key: "=", code: "Equal" }))).toBe("in");
    expect(getFontZoomDirection(keyEvent({ key: "+", code: "Equal" }))).toBe("in");
    expect(getFontZoomDirection(keyEvent({ key: "+", code: "NumpadAdd" }))).toBe("in");
    expect(getFontZoomDirection(keyEvent({ ctrlKey: false, metaKey: true, key: "=" }))).toBe(
      "in",
    );
  });

  test("detects zoom-out shortcuts", () => {
    expect(getFontZoomDirection(keyEvent({ key: "-", code: "Minus" }))).toBe("out");
    expect(getFontZoomDirection(keyEvent({ key: "-", code: "NumpadSubtract" }))).toBe("out");
  });

  test("requires a primary modifier and ignores alt-layered shortcuts", () => {
    expect(getFontZoomDirection(keyEvent({ ctrlKey: false, key: "=", code: "Equal" }))).toBeNull();
    expect(getFontZoomDirection(keyEvent({ altKey: true, key: "=", code: "Equal" }))).toBeNull();
  });

  test("steps font sizes inside the configured bounds", () => {
    expect(nextFontSize(14, "in")).toBe(15);
    expect(nextFontSize(14, "out")).toBe(13);
    expect(nextFontSize(FONT_ZOOM_MAX, "in")).toBe(FONT_ZOOM_MAX);
    expect(nextFontSize(FONT_ZOOM_MIN, "out")).toBe(FONT_ZOOM_MIN);
  });
});
