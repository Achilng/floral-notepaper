import { describe, expect, test, vi } from "vitest";
import {
  NOTE_SURFACE_ACTION_EVENT,
  isNoteSurfaceAction,
  requestSurfaceAction,
  surfaceActionFromEvent,
} from "./surfaceActions";

describe("note surface actions", () => {
  test("accepts only supported note surface actions", () => {
    expect(isNoteSurfaceAction("copy")).toBe(true);
    expect(isNoteSurfaceAction("save")).toBe(true);
    expect(isNoteSurfaceAction("switchToPad")).toBe(true);
    expect(isNoteSurfaceAction("close")).toBe(true);
    expect(isNoteSurfaceAction("delete")).toBe(false);
  });

  test("extracts a note surface action from a custom event", () => {
    const event = new CustomEvent(NOTE_SURFACE_ACTION_EVENT, {
      detail: { action: "save" },
    });

    expect(surfaceActionFromEvent(event)).toBe("save");
    expect(surfaceActionFromEvent(new Event("other"))).toBeNull();
  });

  test("dispatches note surface actions", () => {
    const originalWindow = globalThis.window;
    const eventTarget = new EventTarget();
    Object.defineProperty(globalThis, "window", {
      value: eventTarget,
      configurable: true,
    });
    const listener = vi.fn();
    window.addEventListener(NOTE_SURFACE_ACTION_EVENT, listener);

    requestSurfaceAction("close");

    expect(listener).toHaveBeenCalledTimes(1);
    expect(surfaceActionFromEvent(listener.mock.calls[0][0])).toBe("close");

    window.removeEventListener(NOTE_SURFACE_ACTION_EVENT, listener);
    Object.defineProperty(globalThis, "window", {
      value: originalWindow,
      configurable: true,
    });
  });
});
