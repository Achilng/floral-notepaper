import { invoke } from "@tauri-apps/api/core";
import { openUrl } from "@tauri-apps/plugin-opener";

export interface UpdateInfo {
  version: string;
  body: string | null;
  url: string;
}

export async function checkForUpdate(): Promise<UpdateInfo | null> {
  try {
    return await invoke<UpdateInfo | null>("check_update");
  } catch {
    return null;
  }
}

export async function openReleasePage(url: string): Promise<void> {
  await openUrl(url);
}
