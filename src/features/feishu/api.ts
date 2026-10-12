import { invoke } from "@tauri-apps/api/core";

export interface FeishuSettings {
  enabled: boolean;
  appId: string;
  folderToken: string;
  hasSecret: boolean;
  connected: boolean;
}

export interface FeishuDeviceAuthorization {
  deviceCode: string;
  userCode: string;
  verificationUri: string;
  verificationUriComplete: string;
  expiresIn: number;
  interval: number;
}

export interface FeishuDeviceAuthorizationPoll {
  pending: boolean;
  connected: boolean;
}

export interface FeishuSyncOverview {
  enabled: boolean;
  configured: boolean;
  total: number;
  synced: number;
  pending: number;
  failed: number;
  syncing: boolean;
  lastSyncedAt?: string;
  lastError?: string;
}

export interface FeishuSyncSummary {
  total: number;
  synced: number;
  skipped: number;
  failed: number;
}

export function getFeishuSettings(): Promise<FeishuSettings> {
  return invoke("feishu_settings_get");
}

export function saveFeishuSettings(
  settings: FeishuSettings,
  appSecret?: string,
): Promise<FeishuSettings> {
  return invoke("feishu_settings_save", { settings, appSecret });
}

export function clearFeishuSecret(): Promise<FeishuSettings> {
  return invoke("feishu_secret_clear");
}

export function testFeishuConnection(): Promise<void> {
  return invoke("feishu_connection_test");
}

export function startFeishuAuthorization(): Promise<FeishuDeviceAuthorization> {
  return invoke("feishu_auth_start");
}

export function pollFeishuAuthorization(
  deviceCode: string,
): Promise<FeishuDeviceAuthorizationPoll> {
  return invoke("feishu_auth_poll", { deviceCode });
}

export function getFeishuSyncStatus(): Promise<FeishuSyncOverview> {
  return invoke("feishu_sync_status");
}

export function syncAllNotesToFeishu(): Promise<FeishuSyncSummary> {
  return invoke("feishu_sync_all");
}
