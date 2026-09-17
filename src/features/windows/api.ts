import { invoke } from "@tauri-apps/api/core";

export interface WindowBounds {
  x: number;
  y: number;
  width: number;
  height: number;
}

export function openNotepadWindow(noteId?: string, bounds?: WindowBounds): Promise<string> {
  return invoke("open_notepad_window", {
    noteId: noteId ?? null,
    bounds: bounds ?? null,
  });
}

export function openTileWindow(noteId: string, bounds?: WindowBounds): Promise<string> {
  return invoke("open_tile_window", { noteId, bounds: bounds ?? null });
}

export function toggleTileWindow(noteId: string, bounds?: WindowBounds): Promise<boolean> {
  return invoke("toggle_tile_window", { noteId, bounds: bounds ?? null });
}

export function openNoteInEditor(noteId: string): Promise<void> {
  return invoke("open_note_in_editor", { noteId });
}

export function takeStartupFile(): Promise<string | null> {
  return invoke("take_startup_file");
}

// 前端完成首屏初始状态渲染（含侧边栏收起等设置）后调用，避免主窗口
// 带着默认状态先显示一瞬间、再跳变成配置里的状态
export function appReady(): Promise<void> {
  return invoke("app_ready");
}
