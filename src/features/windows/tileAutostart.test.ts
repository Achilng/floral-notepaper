import { describe, expect, test, vi, beforeEach, afterEach } from "vitest";

const invokeMock = vi.fn();

vi.mock("@tauri-apps/api/core", () => ({
  invoke: (...args: unknown[]) => invokeMock(...args),
}));

import {
  getCurrentTileAutostart,
  loadTileAutostart,
  setCurrentTileAutostart,
  subscribeTileAutostart,
  toggleTileAutostart,
} from "./tileAutostart";

let restoreWindow: () => void;

beforeEach(() => {
  const originalWindow = globalThis.window;
  const eventTarget = new EventTarget();
  Object.defineProperty(globalThis, "window", {
    value: eventTarget,
    configurable: true,
  });
  restoreWindow = () => {
    Object.defineProperty(globalThis, "window", {
      value: originalWindow,
      configurable: true,
    });
  };
});

afterEach(() => {
  restoreWindow();
});

describe("tileAutostart", () => {
  beforeEach(() => {
    invokeMock.mockReset();
    setCurrentTileAutostart(false);
  });

  test("loadTileAutostart syncs shared state from backend", async () => {
    invokeMock.mockResolvedValue(true);
    const enabled = await loadTileAutostart("note-1");

    expect(enabled).toBe(true);
    expect(getCurrentTileAutostart()).toBe(true);
    expect(invokeMock).toHaveBeenCalledWith("get_tile_autostart", { noteId: "note-1" });
  });

  test("loadTileAutostart keeps current state on backend failure", async () => {
    setCurrentTileAutostart(true);
    invokeMock.mockRejectedValue(new Error("boom"));
    const enabled = await loadTileAutostart("note-1");

    expect(enabled).toBe(true);
    expect(getCurrentTileAutostart()).toBe(true);
  });

  test("toggleTileAutostart flips state and uses backend result", async () => {
    invokeMock.mockResolvedValue(["note-1"]);
    const enabled = await toggleTileAutostart("note-1");

    expect(enabled).toBe(true);
    expect(getCurrentTileAutostart()).toBe(true);
    expect(invokeMock).toHaveBeenCalledWith("set_tile_autostart", {
      noteId: "note-1",
      enabled: true,
    });
  });

  test("toggleTileAutostart derives enabled from returned list", async () => {
    setCurrentTileAutostart(true);
    invokeMock.mockResolvedValue([]);
    const enabled = await toggleTileAutostart("note-1");

    expect(enabled).toBe(false);
    expect(getCurrentTileAutostart()).toBe(false);
  });

  test("setCurrentTileAutostart notifies subscribers", () => {
    const seen: boolean[] = [];
    const unsubscribe = subscribeTileAutostart((enabled) => seen.push(enabled));

    setCurrentTileAutostart(true);
    unsubscribe();
    setCurrentTileAutostart(false);

    expect(seen).toEqual([true]);
  });
});
