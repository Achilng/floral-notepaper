import type { WindowBounds } from "./api";

export type NoteSurfaceMode = "pad" | "tile";

export const NOTE_SURFACE_MODE_EVENT = "floral-notepaper:surface-mode";

export const SURFACE_WINDOW_SIZES: Record<
  NoteSurfaceMode,
  Pick<WindowBounds, "width" | "height">
> = {
  pad: { width: 260, height: 260 },
  tile: { width: 260, height: 260 },
};

export function isNoteSurfaceMode(value: unknown): value is NoteSurfaceMode {
  return value === "pad" || value === "tile";
}

/** 编辑模式比磁贴多出顶部页签栏和底部状态栏；便签保持紧凑，只补这些（逻辑像素）。 */
export const PAD_SURFACE_MIN_WIDTH = 260;
export const PAD_SURFACE_EXTRA_HEIGHT = 40;

export interface SurfaceWorkArea {
  x: number;
  y: number;
  width: number;
  height: number;
}

export interface SurfaceBoundsContext {
  /** 设备像素比；补的尺寸按逻辑像素算，窗口边界是物理像素 */
  scale?: number;
  /** 当前显示器工作区（物理像素），用来判断往哪边撑才不出屏幕 */
  workArea?: SurfaceWorkArea | null;
}

/** 切换后的目标边界，以及进入编辑前的快照（退回磁贴时整体还原）。 */
export interface SurfaceBoundsResult {
  target: WindowBounds;
  resting: WindowBounds | null;
}

export function getSurfaceTargetBounds(
  mode: NoteSurfaceMode,
  current: WindowBounds,
  context: SurfaceBoundsContext = {},
  resting: WindowBounds | null = null,
): SurfaceBoundsResult {
  // 退回磁贴：有快照就整体还原，否则保持当前
  if (mode === "tile") return { target: resting ?? current, resting: null };

  const scale = context.scale ?? 1;
  const area = context.workArea ?? null;
  const width = Math.max(current.width, Math.round(PAD_SURFACE_MIN_WIDTH * scale));
  const height = current.height + Math.round(PAD_SURFACE_EXTRA_HEIGHT * scale);
  const growWidth = width - current.width;
  const growHeight = height - current.height;
  // 右边放不下就向左撑、上方放不下就向下撑，别把窗口顶出屏幕
  const outOfRight = area != null && current.x + width > area.x + area.width;
  const outOfTop = area != null && current.y - growHeight < area.y;

  return {
    target: {
      x: outOfRight ? current.x - growWidth : current.x,
      y: outOfTop ? current.y : current.y - growHeight,
      width,
      height,
    },
    resting: { ...current },
  };
}

export function requestSurfaceMode(mode: NoteSurfaceMode): void {
  window.dispatchEvent(new CustomEvent(NOTE_SURFACE_MODE_EVENT, { detail: { mode } }));
}

export function surfaceModeFromEvent(event: Event): NoteSurfaceMode | null {
  if (!(event instanceof CustomEvent)) return null;
  const mode = (event.detail as { mode?: unknown } | null)?.mode;
  return isNoteSurfaceMode(mode) ? mode : null;
}
