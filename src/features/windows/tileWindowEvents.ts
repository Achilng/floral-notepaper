import { emit } from "@tauri-apps/api/event";
import type { NoteSurfaceMode } from "./surfaceMode";

export const TILE_WINDOW_CLOSED_EVENT = "tile-window-closed";
export const TILE_WINDOW_UNPINNED_EVENT = "tile-window-unpinned";

export function syncPinnedTileIds(
  current: Set<string>,
  noteId: string,
  pinned: boolean,
): Set<string> {
  const next = new Set(current);
  if (pinned) {
    next.add(noteId);
  } else {
    next.delete(noteId);
  }
  return next;
}

export function tileSurfaceModeUnpinNoteId(
  currentMode: NoteSurfaceMode,
  nextMode: NoteSurfaceMode,
  noteId: string,
): string | null {
  return currentMode === "tile" && nextMode === "pad" && noteId ? noteId : null;
}

export function emitTileWindowUnpinned(noteId: string): Promise<void> {
  return emit(TILE_WINDOW_UNPINNED_EVENT, noteId);
}
