import type { WindowBounds } from "./api";

export type NoteSurfaceMode = "pad" | "tile";

export const NOTE_SURFACE_MODE_EVENT = "huajian:surface-mode";

export const SURFACE_WINDOW_SIZES: Record<
  NoteSurfaceMode,
  Pick<WindowBounds, "width" | "height">
> = {
  pad: { width: 420, height: 430 },
  tile: { width: 420, height: 430 },
};

export function isNoteSurfaceMode(value: unknown): value is NoteSurfaceMode {
  return value === "pad" || value === "tile";
}

export function getSurfaceTargetBounds(
  _mode: NoteSurfaceMode,
  current: WindowBounds,
): WindowBounds {
  return current;
}

export function requestSurfaceMode(mode: NoteSurfaceMode): void {
  window.dispatchEvent(
    new CustomEvent(NOTE_SURFACE_MODE_EVENT, { detail: { mode } }),
  );
}

export function surfaceModeFromEvent(event: Event): NoteSurfaceMode | null {
  if (!(event instanceof CustomEvent)) return null;
  const mode = (event.detail as { mode?: unknown } | null)?.mode;
  return isNoteSurfaceMode(mode) ? mode : null;
}
