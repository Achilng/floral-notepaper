import { invoke } from "@tauri-apps/api/core";

export interface WindowBounds {
  x: number;
  y: number;
  width: number;
  height: number;
}

export function openNotepadWindow(
  noteId?: string,
  bounds?: WindowBounds,
): Promise<string> {
  return invoke("open_notepad_window", {
    noteId: noteId ?? null,
    bounds: bounds ?? null,
  });
}

export function openTileWindow(
  noteId: string,
  bounds?: WindowBounds,
): Promise<string> {
  return invoke("open_tile_window", { noteId, bounds: bounds ?? null });
}
