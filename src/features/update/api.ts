import { invoke } from "@tauri-apps/api/core";
import type {
  DownloadSourceUsed,
  UpdateCheckResult,
  UpdateDownloadResult,
  UpdateInstallPrepareReportStatus,
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

export function reportInstallPreparation(
  requestId: string,
  windowLabel: string,
  status: UpdateInstallPrepareReportStatus,
  message?: string,
): Promise<void> {
  return invoke("update_install_prepare_report", {
    requestId,
    windowLabel,
    status,
    message,
  });
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

export function setMirrorChyanCdk(cdk: string): Promise<void> {
  return invoke("update_mirror_chyan_cdk_set", { cdk });
}

export function getMirrorChyanCdk(): Promise<string | null> {
  return invoke("update_mirror_chyan_cdk_get");
}

export function clearMirrorChyanCdk(): Promise<void> {
  return invoke("update_mirror_chyan_cdk_clear");
}
