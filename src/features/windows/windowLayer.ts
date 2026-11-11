import { getCurrentWindow } from "@tauri-apps/api/window";

export type WindowLayer = "top" | "normal" | "bottom";

export const DEFAULT_WINDOW_LAYER: WindowLayer = "top";

export const WINDOW_LAYER_CHANGED_EVENT = "floral-notepaper:window-layer-changed";

const WINDOW_LAYERS: WindowLayer[] = ["top", "normal", "bottom"];
const STORAGE_PREFIX = "floral-notepaper:window-layer:";

export function isWindowLayer(value: unknown): value is WindowLayer {
  return value === "top" || value === "normal" || value === "bottom";
}

export function nextWindowLayer(current: WindowLayer): WindowLayer {
  const index = WINDOW_LAYERS.indexOf(current);
  return WINDOW_LAYERS[(index + 1) % WINDOW_LAYERS.length] ?? DEFAULT_WINDOW_LAYER;
}

export function tileLayerStorageScope(noteId: string): string {
  return `tile:${noteId}`;
}

export function readStoredWindowLayer(scope: string): WindowLayer {
  try {
    const raw = window.localStorage.getItem(STORAGE_PREFIX + scope);
    return isWindowLayer(raw) ? raw : DEFAULT_WINDOW_LAYER;
  } catch {
    return DEFAULT_WINDOW_LAYER;
  }
}

export function writeStoredWindowLayer(scope: string, layer: WindowLayer): void {
  try {
    window.localStorage.setItem(STORAGE_PREFIX + scope, layer);
  } catch {
    // localStorage 不可用时仅丢失持久化，不影响当前会话
  }
}

let currentLayer: WindowLayer = DEFAULT_WINDOW_LAYER;

export function getCurrentWindowLayer(): WindowLayer {
  return currentLayer;
}

export function setCurrentWindowLayer(layer: WindowLayer): void {
  currentLayer = layer;
  window.dispatchEvent(new CustomEvent(WINDOW_LAYER_CHANGED_EVENT, { detail: { layer } }));
}

export function subscribeWindowLayer(handler: (layer: WindowLayer) => void): () => void {
  const listener = (event: Event) => {
    const layer = (event as CustomEvent<{ layer?: unknown }>).detail?.layer;
    if (isWindowLayer(layer)) handler(layer);
  };
  window.addEventListener(WINDOW_LAYER_CHANGED_EVENT, listener);
  return () => window.removeEventListener(WINDOW_LAYER_CHANGED_EVENT, listener);
}

/**
 * 将层级应用到当前窗口。macOS 不支持 always-on-bottom，切换到
 * bottom 会抛错，由调用方决定是否提示。
 */
export async function applyWindowLayer(layer: WindowLayer): Promise<void> {
  const current = getCurrentWindow();

  if (layer === "bottom") {
    await current.setAlwaysOnTop(false);
    await current.setAlwaysOnBottom(true);
    return;
  }

  try {
    await current.setAlwaysOnBottom(false);
  } catch {
    // 平台不支持 always-on-bottom 时忽略清除失败
  }
  await current.setAlwaysOnTop(layer === "top");
}
