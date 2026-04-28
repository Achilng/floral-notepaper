import { getCurrentWindow } from "@tauri-apps/api/window";

export function closeCurrentWindow(): Promise<void> {
  return getCurrentWindow().close();
}

export function minimizeCurrentWindow(): Promise<void> {
  return getCurrentWindow().minimize();
}

export function toggleMaximizeCurrentWindow(): Promise<void> {
  return getCurrentWindow().toggleMaximize();
}

export function isCurrentWindowMaximized(): Promise<boolean> {
  return getCurrentWindow().isMaximized();
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
