import { describe, expect, test } from "vitest";
import { getViewportPopupPosition } from "./popupPosition";

const viewport = { width: 900, height: 620 };

describe("getViewportPopupPosition", () => {
  test("keeps the pointer position when the popup fits", () => {
    expect(
      getViewportPopupPosition({ x: 240, y: 180 }, { width: 168, height: 104 }, viewport),
    ).toEqual({ x: 240, y: 180 });
  });

  test("opens above and to the left of the pointer near the bottom-right corner", () => {
    expect(
      getViewportPopupPosition({ x: 880, y: 600 }, { width: 168, height: 104 }, viewport),
    ).toEqual({ x: 712, y: 496 });
  });

  test("uses the measured height when a popup panel grows", () => {
    expect(
      getViewportPopupPosition({ x: 240, y: 590 }, { width: 168, height: 400 }, viewport),
    ).toEqual({ x: 240, y: 190 });
  });

  test("keeps an oversized popup inside the viewport margin", () => {
    expect(
      getViewportPopupPosition({ x: 450, y: 310 }, { width: 1000, height: 700 }, viewport),
    ).toEqual({ x: 4, y: 4 });
  });
});
