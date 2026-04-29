import { describe, expect, test } from "vitest";
import { DEFAULT_TILE_COLOR, normalizeTileColor } from "./tileColor";

describe("tile color settings", () => {
  test("normalizes full and shorthand hex colors", () => {
    expect(normalizeTileColor("#ABCDEF")).toBe("#abcdef");
    expect(normalizeTileColor("abc")).toBe("#aabbcc");
  });

  test("falls back to the default tile color for invalid values", () => {
    expect(DEFAULT_TILE_COLOR).toBe("#f6f3ec");
    expect(normalizeTileColor("")).toBe(DEFAULT_TILE_COLOR);
    expect(normalizeTileColor("#12zz99")).toBe(DEFAULT_TILE_COLOR);
  });
});
