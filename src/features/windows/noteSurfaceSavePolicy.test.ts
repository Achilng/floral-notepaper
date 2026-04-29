import { describe, expect, test } from "vitest";
import { shouldSaveBeforeSwitchingToTile } from "./noteSurfaceSavePolicy";

describe("note surface save policy", () => {
  test("only saves before switching to tile when auto-save is enabled", () => {
    expect(shouldSaveBeforeSwitchingToTile(true)).toBe(true);
    expect(shouldSaveBeforeSwitchingToTile(false)).toBe(false);
  });
});
