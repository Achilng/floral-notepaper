import { describe, expect, test } from "vitest";
import { formatHeldKeys, hotkeyToConfigString, isValidGlobalShortcut } from "./shortcutRecorder";

describe("shortcutRecorder", () => {
  test("serializes meta shortcuts into config strings", () => {
    const layeredMetaShortcut = "Meta+Shift+P" as Parameters<typeof hotkeyToConfigString>[0];

    expect(hotkeyToConfigString("Meta+K")).toBe("Meta+K");
    expect(hotkeyToConfigString(layeredMetaShortcut)).toBe("Shift+Meta+P");
  });

  test("accepts meta as a valid global shortcut modifier", () => {
    expect(isValidGlobalShortcut("Meta+K")).toBe(true);
    expect(isValidGlobalShortcut("Shift+K")).toBe(false);
  });

  test("formats held meta keys for the recorder UI", () => {
    expect(formatHeldKeys(["Meta", "P"])).toBe("Meta + P");
  });
});
