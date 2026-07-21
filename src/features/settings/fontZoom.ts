export type FontZoomDirection = "in" | "out";

export const FONT_ZOOM_MIN = 8;
export const FONT_ZOOM_MAX = 30;
export const FONT_ZOOM_STEP = 1;
export const FONT_ZOOM_DEFAULT = 14;

interface FontZoomKeyboardEvent {
  altKey: boolean;
  ctrlKey: boolean;
  metaKey: boolean;
  key: string;
  code: string;
}

const ZOOM_IN_KEYS = new Set(["=", "+", "Add"]);
const ZOOM_IN_CODES = new Set(["Equal", "NumpadAdd"]);
const ZOOM_OUT_KEYS = new Set(["-", "Subtract"]);
const ZOOM_OUT_CODES = new Set(["Minus", "NumpadSubtract"]);

export function getFontZoomDirection(event: FontZoomKeyboardEvent): FontZoomDirection | null {
  if (!(event.ctrlKey || event.metaKey)) return null;
  if (event.altKey) return null;

  if (ZOOM_IN_KEYS.has(event.key) || ZOOM_IN_CODES.has(event.code)) return "in";
  if (ZOOM_OUT_KEYS.has(event.key) || ZOOM_OUT_CODES.has(event.code)) return "out";

  return null;
}

export function clampFontSize(value: number): number {
  if (!Number.isFinite(value)) return FONT_ZOOM_DEFAULT;
  return Math.min(FONT_ZOOM_MAX, Math.max(FONT_ZOOM_MIN, Math.round(value)));
}

export function nextFontSize(current: number, direction: FontZoomDirection): number {
  const baseSize = clampFontSize(current);
  const delta = direction === "in" ? FONT_ZOOM_STEP : -FONT_ZOOM_STEP;
  return clampFontSize(baseSize + delta);
}

export function isFontZoomEventTargetBlocked(target: EventTarget | null): boolean {
  return (
    target instanceof HTMLElement &&
    Boolean(target.closest('[data-shortcut-recorder-recording="true"]'))
  );
}
