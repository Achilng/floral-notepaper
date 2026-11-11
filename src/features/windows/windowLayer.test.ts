import { afterEach, beforeEach, describe, expect, test, vi } from "vitest";
import {
  DEFAULT_WINDOW_LAYER,
  WINDOW_LAYER_CHANGED_EVENT,
  getCurrentWindowLayer,
  isWindowLayer,
  nextWindowLayer,
  readStoredWindowLayer,
  setCurrentWindowLayer,
  subscribeWindowLayer,
  tileLayerStorageScope,
  writeStoredWindowLayer,
} from "./windowLayer";

class MemoryStorage {
  private map = new Map<string, string>();

  getItem(key: string): string | null {
    return this.map.get(key) ?? null;
  }

  setItem(key: string, value: string): void {
    this.map.set(key, value);
  }
}

let restoreWindow: () => void;

beforeEach(() => {
  const originalWindow = globalThis.window;
  const eventTarget = new EventTarget();
  const storage = new MemoryStorage();
  Object.defineProperty(globalThis, "window", {
    value: Object.assign(eventTarget, { localStorage: storage }),
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

describe("window layer", () => {
  test("accepts only supported window layers", () => {
    expect(isWindowLayer("top")).toBe(true);
    expect(isWindowLayer("normal")).toBe(true);
    expect(isWindowLayer("bottom")).toBe(true);
    expect(isWindowLayer("middle")).toBe(false);
    expect(isWindowLayer(undefined)).toBe(false);
  });

  test("cycles through layers in order", () => {
    expect(nextWindowLayer("top")).toBe("normal");
    expect(nextWindowLayer("normal")).toBe("bottom");
    expect(nextWindowLayer("bottom")).toBe("top");
  });

  test("defaults to top when storage is empty", () => {
    expect(readStoredWindowLayer(tileLayerStorageScope("note-1"))).toBe(DEFAULT_WINDOW_LAYER);
  });

  test("persists and reads back a layer per scope", () => {
    const scope = tileLayerStorageScope("note-1");
    writeStoredWindowLayer(scope, "bottom");
    expect(readStoredWindowLayer(scope)).toBe("bottom");

    writeStoredWindowLayer(scope, "normal");
    expect(readStoredWindowLayer(scope)).toBe("normal");

    expect(readStoredWindowLayer(tileLayerStorageScope("note-2"))).toBe(DEFAULT_WINDOW_LAYER);
  });

  test("falls back to default for unknown stored values", () => {
    window.localStorage.setItem("floral-notepaper:window-layer:tile:note-1", "sideways");
    expect(readStoredWindowLayer(tileLayerStorageScope("note-1"))).toBe(DEFAULT_WINDOW_LAYER);
  });
});

describe("window layer store", () => {
  test("notifies subscribers when the current layer changes", () => {
    const handler = vi.fn();
    const unsubscribe = subscribeWindowLayer(handler);

    setCurrentWindowLayer("bottom");

    expect(getCurrentWindowLayer()).toBe("bottom");
    expect(handler).toHaveBeenCalledTimes(1);
    expect(handler).toHaveBeenCalledWith("bottom");

    setCurrentWindowLayer("top");
    expect(handler).toHaveBeenCalledWith("top");

    unsubscribe();
    setCurrentWindowLayer("normal");
    expect(handler).toHaveBeenCalledTimes(2);
  });

  test("ignores layer change events without a valid layer payload", () => {
    const handler = vi.fn();
    const unsubscribe = subscribeWindowLayer(handler);

    window.dispatchEvent(new CustomEvent(WINDOW_LAYER_CHANGED_EVENT, { detail: {} }));
    expect(handler).toHaveBeenCalledTimes(0);

    unsubscribe();
  });
});
