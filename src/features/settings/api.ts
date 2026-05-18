import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import type { AppConfig, ThemeOption, TileColorMode, ViewMode } from "./types";

export type { AppConfig, ThemeOption, TileColorMode, ViewMode };
export { DEFAULT_TILE_COLOR, normalizeTileColor } from "./tileColor";
export { applyTheme, watchSystemTheme } from "./theme";

export const supportedShortcuts = [
  "Ctrl+Space",
  "Alt+Space",
  "Ctrl+Shift+Space",
  "Ctrl+K",
  "Ctrl+N",
  "Ctrl+Shift+N",
  "Alt+Shift+Space",
  "Ctrl+Alt+K",
  "Super+Space",
  "Ctrl+T",
  "Ctrl+Enter",
  "Ctrl+Escape",
] as const;

export function getConfig(): Promise<AppConfig> {
  return invoke("config_get");
}

export function saveConfig(config: AppConfig): Promise<AppConfig> {
  return invoke("config_save", { config });
}

export async function chooseNotesDirectory(): Promise<string | null> {
  const path = await open({
    directory: true,
    multiple: false,
  });

  return typeof path === "string" ? path : null;
}

export function normalizeViewMode(value: string): ViewMode {
  if (value === "edit" || value === "split" || value === "preview") {
    return value;
  }

  return "split";
}
