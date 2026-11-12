import { invoke } from "@tauri-apps/api/core";

export const TILE_AUTOSTART_CHANGED_EVENT = "floral-notepaper:tile-autostart-changed";

let currentTileAutostart = false;

export function getCurrentTileAutostart(): boolean {
  return currentTileAutostart;
}

export function setCurrentTileAutostart(enabled: boolean): void {
  currentTileAutostart = enabled;
  window.dispatchEvent(new CustomEvent(TILE_AUTOSTART_CHANGED_EVENT, { detail: { enabled } }));
}

export function subscribeTileAutostart(handler: (enabled: boolean) => void): () => void {
  const listener = (event: Event) => {
    const enabled = (event as CustomEvent<{ enabled?: unknown }>).detail?.enabled;
    if (typeof enabled === "boolean") handler(enabled);
  };
  window.addEventListener(TILE_AUTOSTART_CHANGED_EVENT, listener);
  return () => window.removeEventListener(TILE_AUTOSTART_CHANGED_EVENT, listener);
}

/** 查询当前磁贴的开机自启状态并写入共享状态 */
export async function loadTileAutostart(noteId: string): Promise<boolean> {
  try {
    const enabled = await invoke<boolean>("get_tile_autostart", { noteId });
    setCurrentTileAutostart(enabled);
    return enabled;
  } catch {
    return currentTileAutostart;
  }
}

/** 切换当前磁贴的开机自启状态，返回切换后的值 */
export async function toggleTileAutostart(noteId: string): Promise<boolean> {
  const next = !currentTileAutostart;
  const tiles = await invoke<string[]>("set_tile_autostart", {
    noteId,
    enabled: next,
  });
  const enabled = tiles.includes(noteId);
  setCurrentTileAutostart(enabled);
  return enabled;
}
