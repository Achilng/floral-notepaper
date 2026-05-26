import { invoke } from "@tauri-apps/api/core";
import type {
  DownloadSourceUsed,
  UpdateCheckResult,
  UpdateDownloadResult,
  UpdateInstallResult,
  UpdateSettings,
  UpdateState,
} from "./types";

export function checkForUpdates(manual: boolean): Promise<UpdateCheckResult> {
  return invoke("update_check", { manual });
}

export function downloadUpdate(source?: DownloadSourceUsed): Promise<UpdateDownloadResult> {
  return invoke("update_download", { source });
}

export function installUpdate(): Promise<UpdateInstallResult> {
  return invoke("update_install");
}

export function cancelUpdate(): Promise<void> {
  return invoke("update_cancel");
}

export function getUpdateStatus(): Promise<UpdateState> {
  return invoke("update_status");
}

export function getUpdateSettings(): Promise<UpdateSettings> {
  return invoke("update_settings_get");
}

export function saveUpdateSettings(settings: UpdateSettings): Promise<UpdateSettings> {
  return invoke("update_settings_save", { settings });
}

export function setMirrorCdk(cdk: string): Promise<void> {
  return invoke("update_mirror_cdk_set", { cdk });
}

export function clearMirrorCdk(): Promise<void> {
  return invoke("update_mirror_cdk_clear");
}
