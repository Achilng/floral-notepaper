import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import { beforeEach, describe, expect, test, vi } from "vitest";
import {
  chooseNotesDirectory,
  getConfig,
  normalizeViewMode,
  saveConfig,
  supportedShortcuts,
} from "./api";
import type { AppConfig } from "./types";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

vi.mock("@tauri-apps/plugin-dialog", () => ({
  open: vi.fn(),
}));

const mockedInvoke = vi.mocked(invoke);
const mockedOpen = vi.mocked(open);

describe("settings api", () => {
  beforeEach(() => {
    mockedInvoke.mockReset();
    mockedOpen.mockReset();
  });

  test("gets config through Rust", async () => {
    const config: AppConfig = {
      notesDir: "D:\\notes",
      globalShortcut: "Ctrl+Space",
      closeToTray: true,
      autostart: false,
      defaultViewMode: "split",
    };
    mockedInvoke.mockResolvedValue(config);

    await expect(getConfig()).resolves.toBe(config);

    expect(invoke).toHaveBeenCalledWith("config_get");
  });

  test("saves config through Rust", async () => {
    const config: AppConfig = {
      notesDir: "D:\\notes",
      globalShortcut: "Alt+Space",
      closeToTray: false,
      autostart: true,
      defaultViewMode: "preview",
    };
    mockedInvoke.mockResolvedValue(config);

    await expect(saveConfig(config)).resolves.toBe(config);

    expect(invoke).toHaveBeenCalledWith("config_save", { config });
  });

  test("normalizes supported view modes and falls back to split", () => {
    expect(normalizeViewMode("edit")).toBe("edit");
    expect(normalizeViewMode("split")).toBe("split");
    expect(normalizeViewMode("preview")).toBe("preview");
    expect(normalizeViewMode("unknown")).toBe("split");
  });

  test("lists supported global shortcuts", () => {
    expect(supportedShortcuts).toEqual(["Ctrl+Space", "Alt+Space"]);
  });

  test("chooses a notes directory through the folder picker", async () => {
    mockedOpen.mockResolvedValue("D:\\notes");

    await expect(chooseNotesDirectory()).resolves.toBe("D:\\notes");

    expect(open).toHaveBeenCalledWith({
      directory: true,
      multiple: false,
    });
  });

  test("returns null when choosing a notes directory is cancelled", async () => {
    mockedOpen.mockResolvedValue(null);

    await expect(chooseNotesDirectory()).resolves.toBeNull();
  });
});
