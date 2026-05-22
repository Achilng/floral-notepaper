import { describe, expect, test } from "vitest";
import { formatHeldKeys, hotkeyToConfigString, isValidGlobalShortcut } from "./shortcutRecorder";

describe("shortcutRecorder", () => {
  test("serializes shortcuts into Windows config strings", () => {
    const layeredMetaShortcut = "Meta+Shift+P" as Parameters<typeof hotkeyToConfigString>[0];

    expect(hotkeyToConfigString("Meta+K")).toBe("K");
    expect(hotkeyToConfigString(layeredMetaShortcut)).toBe("Shift+P");
  });

  test("requires ctrl or alt as a valid global shortcut modifier", () => {
    expect(isValidGlobalShortcut("Meta+K")).toBe(false);
    expect(isValidGlobalShortcut("Shift+K")).toBe(false);
  });

  test("formats held meta keys with Windows labels for the recorder UI", () => {
    expect(formatHeldKeys(["Meta", "P"])).toBe("Win + P");
  });
});
