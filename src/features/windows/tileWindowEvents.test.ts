import { emit } from "@tauri-apps/api/event";
import { beforeEach, describe, expect, test, vi } from "vitest";
import {
  TILE_WINDOW_UNPINNED_EVENT,
  emitTileWindowUnpinned,
  syncPinnedTileIds,
  tileSurfaceModeUnpinNoteId,
} from "./tileWindowEvents";

vi.mock("@tauri-apps/api/event", () => ({
  emit: vi.fn(),
}));

describe("tile window events", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  test("removes a note from the pinned set when its tile becomes a pad", () => {
    const next = syncPinnedTileIds(new Set(["note-1", "note-2"]), "note-1", false);

    expect([...next]).toEqual(["note-2"]);
  });

  test("detects only tile to pad transitions as unpin events", () => {
    expect(tileSurfaceModeUnpinNoteId("tile", "pad", "note-1")).toBe("note-1");
    expect(tileSurfaceModeUnpinNoteId("pad", "tile", "note-1")).toBeNull();
    expect(tileSurfaceModeUnpinNoteId("tile", "tile", "note-1")).toBeNull();
    expect(tileSurfaceModeUnpinNoteId("tile", "pad", "")).toBeNull();
  });

  test("emits a global unpin event for the main window", async () => {
    await emitTileWindowUnpinned("note-1");

    expect(emit).toHaveBeenCalledWith(TILE_WINDOW_UNPINNED_EVENT, "note-1");
  });
});
