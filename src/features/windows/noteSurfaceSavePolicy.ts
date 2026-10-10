import type { NoteSurfaceMode } from "./surfaceMode";

export function shouldSaveBeforeSwitchingToTile(autoSave: boolean): boolean {
  return autoSave;
}

export function shouldEnterPadFromTileOnDoubleClick(
  enabled: boolean,
  isControlTarget: boolean,
): boolean {
  return enabled && !isControlTarget;
}

export function shouldReturnToTileAfterManualSave({
  enabled,
  noteId,
  currentMode,
  isAutoSave,
}: {
  enabled: boolean;
  noteId: string;
  currentMode: NoteSurfaceMode;
  isAutoSave: boolean;
}): boolean {
  return enabled && Boolean(noteId) && currentMode === "pad" && !isAutoSave;
}
