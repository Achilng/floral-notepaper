import { invoke } from "@tauri-apps/api/core";
import { PhysicalPosition, PhysicalSize } from "@tauri-apps/api/dpi";
import { getCurrentWindow } from "@tauri-apps/api/window";
import type { WindowBounds } from "./api";

export type ResizeDirection =
  | "NorthWest"
  | "NorthEast"
  | "SouthWest"
  | "SouthEast";

export function showCurrentWindow(): Promise<void> {
  return getCurrentWindow().show();
}

export function hideCurrentWindow(): Promise<void> {
  return getCurrentWindow().hide();
}

export function closeCurrentWindow(): Promise<void> {
  return getCurrentWindow().close();
}

export function recycleCurrentNotepad(): Promise<void> {
  return invoke("recycle_notepad_window", {
    label: getCurrentWindow().label,
  });
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

export function startCurrentWindowResize(
  direction: ResizeDirection = "SouthEast",
): Promise<void> {
  return getCurrentWindow().startResizeDragging(direction);
}

export async function getCurrentWindowBounds(): Promise<WindowBounds> {
  const window = getCurrentWindow();
  const [position, size] = await Promise.all([
    window.outerPosition(),
    window.innerSize(),
  ]);

  return {
    x: position.x,
    y: position.y,
    width: size.width,
    height: size.height,
  };
}

export async function setCurrentWindowBounds(
  bounds: WindowBounds,
): Promise<void> {
  const window = getCurrentWindow();
  await Promise.all([
    window.setPosition(new PhysicalPosition(bounds.x, bounds.y)),
    window.setSize(new PhysicalSize(bounds.width, bounds.height)),
  ]);
}

export async function animateCurrentWindowBounds(
  target: WindowBounds,
  durationMs = 180,
): Promise<void> {
  const start = await getCurrentWindowBounds();
  const raf = globalThis.requestAnimationFrame;

  if (!raf || durationMs <= 0) {
    await setCurrentWindowBounds(target);
    return;
  }

  await new Promise<void>((resolve, reject) => {
    const startedAt = globalThis.performance?.now() ?? Date.now();

    const step = (timestamp: number) => {
      const elapsed = timestamp - startedAt;
      const progress = Math.min(1, elapsed / durationMs);
      const eased = 1 - Math.pow(1 - progress, 3);

      const next: WindowBounds = {
        x: interpolate(start.x, target.x, eased),
        y: interpolate(start.y, target.y, eased),
        width: interpolate(start.width, target.width, eased),
        height: interpolate(start.height, target.height, eased),
      };

      void setCurrentWindowBounds(next)
        .then(() => {
          if (progress < 1) {
            raf(step);
          } else {
            resolve();
          }
        })
        .catch(reject);
    };

    raf(step);
  });
}

function interpolate(start: number, end: number, progress: number): number {
  return Math.round(start + (end - start) * progress);
}
