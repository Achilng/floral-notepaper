import { getCurrentWindow } from "@tauri-apps/api/window";

export function closeCurrentWindow(): Promise<void> {
  return getCurrentWindow().close();
}

export function setCurrentWindowAlwaysOnTop(enabled: boolean): Promise<void> {
  return getCurrentWindow().setAlwaysOnTop(enabled);
}

export function startCurrentWindowDrag(): Promise<void> {
  return getCurrentWindow().startDragging();
}

export function startCurrentWindowResize(): Promise<void> {
  return getCurrentWindow().startResizeDragging("SouthEast");
}
