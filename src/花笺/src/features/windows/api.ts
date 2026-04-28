import { invoke } from "@tauri-apps/api/core";

export function openNotepadWindow(noteId?: string): Promise<string> {
  return invoke("open_notepad_window", { noteId: noteId ?? null });
}

export function openTileWindow(noteId: string): Promise<string> {
  return invoke("open_tile_window", { noteId });
}
