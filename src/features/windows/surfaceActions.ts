export type NoteSurfaceAction = "copy" | "save" | "switchToPad" | "close";

export const NOTE_SURFACE_ACTION_EVENT = "huajian:surface-action";

export function isNoteSurfaceAction(
  value: unknown,
): value is NoteSurfaceAction {
  return (
    value === "copy" ||
    value === "save" ||
    value === "switchToPad" ||
    value === "close"
  );
}

export function requestSurfaceAction(action: NoteSurfaceAction): void {
  window.dispatchEvent(
    new CustomEvent(NOTE_SURFACE_ACTION_EVENT, { detail: { action } }),
  );
}

export function surfaceActionFromEvent(event: Event): NoteSurfaceAction | null {
  if (!(event instanceof CustomEvent)) return null;
  const action = (event.detail as { action?: unknown } | null)?.action;
  return isNoteSurfaceAction(action) ? action : null;
}
