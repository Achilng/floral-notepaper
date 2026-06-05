import { readFileSync } from "node:fs";
import { describe, expect, test } from "vitest";

describe("Windows installer configuration", () => {
  test("respects the custom install directory selected by the user", () => {
    const config = JSON.parse(
      readFileSync(new URL("../src-tauri/tauri.conf.json", import.meta.url), "utf8"),
    );

    expect(config.bundle.windows.nsis).not.toHaveProperty("installerHooks");
  });
});
