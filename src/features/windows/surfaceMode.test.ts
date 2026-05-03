import { describe, expect, test } from "vitest";
import {
  NOTE_SURFACE_MODE_EVENT,
  SURFACE_WINDOW_SIZES,
  getSurfaceTargetBounds,
  isNoteSurfaceMode,
} from "./surfaceMode";

describe("surface mode helpers", () => {
  test("keeps surface modes explicit", () => {
    expect(isNoteSurfaceMode("pad")).toBe(true);
    expect(isNoteSurfaceMode("tile")).toBe(true);
    expect(isNoteSurfaceMode("main")).toBe(false);
    expect(NOTE_SURFACE_MODE_EVENT).toBe("floral-notepaper:surface-mode");
  });

  test("keeps the current window bounds when switching surface modes", () => {
    const current = {
      x: 100,
      y: 80,
      width: 420,
      height: 430,
    };
    const target = getSurfaceTargetBounds("tile", {
      ...current,
    });

    expect(SURFACE_WINDOW_SIZES.tile).toEqual(SURFACE_WINDOW_SIZES.pad);
    expect(SURFACE_WINDOW_SIZES.pad).toEqual({ width: 260, height: 260 });
    expect(target).toEqual(current);
  });
});
