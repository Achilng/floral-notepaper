import { invoke } from "@tauri-apps/api/core";
import { describe, expect, test, vi } from "vitest";
import { openNotepadWindow, openTileWindow, type WindowBounds } from "./api";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

describe("window api", () => {
  test("passes optional bounds when opening tile and notepad windows", async () => {
    const bounds: WindowBounds = { x: 12, y: 34, width: 320, height: 240 };
    vi.mocked(invoke).mockResolvedValue("ok");

    await openTileWindow("note-1", bounds);
    await openNotepadWindow("note-1", bounds);

    expect(invoke).toHaveBeenNthCalledWith(1, "open_tile_window", {
      noteId: "note-1",
      bounds,
    });
    expect(invoke).toHaveBeenNthCalledWith(2, "open_notepad_window", {
      noteId: "note-1",
      bounds,
    });
  });
});
