import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import type { AppConfig, ViewMode } from "./types";

export interface ShortcutCheckResult {
  available: boolean;
  conflictType: "none" | "current" | "invalid" | "system" | "registered" | "unknown";
  message: string;
}

export function getConfig(): Promise<AppConfig> {
  return invoke("config_get");
}

export function saveConfig(config: AppConfig): Promise<AppConfig> {
  return invoke("config_save", { config });
}

export function checkGlobalShortcut(shortcut: string): Promise<ShortcutCheckResult> {
  return invoke("global_shortcut_check", { shortcut });
}

export async function chooseDataDirectory(): Promise<string | null> {
  const path = await open({
    directory: true,
    multiple: false,
  });

  return typeof path === "string" ? path : null;
}

export function migrateDataDir(newDataDir: string): Promise<AppConfig> {
  return invoke("config_migrate_data_dir", { newDataDir });
}

export async function chooseBackgroundImage(): Promise<string | null> {
  const path = await open({
    directory: false,
    multiple: false,
    filters: [{ name: "Images", extensions: ["png", "jpg", "jpeg", "webp", "gif"] }],
  });

  return typeof path === "string" ? path : null;
}

export function normalizeViewMode(value: string): ViewMode {
  if (value === "edit" || value === "split" || value === "preview") {
    return value;
  }

  return "split";
}
