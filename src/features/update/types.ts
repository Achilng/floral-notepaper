export type CheckSourcePreference = "mirrorChyanFirst" | "githubFirst";

export type DownloadSourcePreference = "mirrorChyanFirst" | "githubFirst";

export type DownloadSourceUsed = "mirrorChyan" | "github";

export type UpdateChannel = "stable" | "beta";

export type UpdateInstallMode = "apply" | "test";

export type UpdateStatus =
  | "idle"
  | "checking"
  | "available"
  | "downloading"
  | "downloaded"
  | "installing"
  | "installScheduled"
  | "failed";

export type UpdateCheckStatus = "notAvailable" | "available" | "failed";

export interface UpdateSettings {
  autoCheck: boolean;
  autoDownload: boolean;
  checkIntervalHours: number;
  checkSourcePreference: CheckSourcePreference;
  downloadSourcePreference: DownloadSourcePreference;
  channel: UpdateChannel;
  allowPrerelease: boolean;
  lastAutoCheckAt?: string | null;
  hasMirrorChyanCdk: boolean;
  mirrorChyanCdkLength?: number | null;
}

export interface UpdateErrorPayload {
  code: string;
  message: string;
  recoverable: boolean;
  action?: string | null;
}

export interface UpdateState {
  status: UpdateStatus;
  currentVersion: string;
  latestVersion?: string | null;
  channel: UpdateChannel;
  assetName?: string | null;
  assetPath?: string | null;
  assetSha256?: string | null;
  assetSize?: number | null;
  assetUrl?: string | null;
  source?: DownloadSourceUsed | null;
  checkedAt?: string | null;
  downloadedAt?: string | null;
  installLogPath?: string | null;
  installMode?: UpdateInstallMode | null;
  installStartedAt?: string | null;
  installScheduledAt?: string | null;
  lastError?: UpdateErrorPayload | null;
}

export interface UpdateCheckResult {
  status: UpdateCheckStatus;
  currentVersion: string;
  latestVersion?: string | null;
  releaseNotes?: string | null;
  mandatory: boolean;
  canDownloadFromMirrorChyan: boolean;
  canDownloadFromGithub: boolean;
  recommendedSource?: DownloadSourceUsed | null;
  assetUrl?: string | null;
}

export interface UpdateDownloadResult {
  status: UpdateStatus;
  version?: string | null;
  assetPath?: string | null;
  source?: DownloadSourceUsed | null;
}

export interface UpdateInstallResult {
  status: UpdateStatus;
  logPath?: string | null;
  mode: UpdateInstallMode;
}

export interface UpdateDownloadProgress {
  version: string;
  assetName: string;
  downloadedBytes: number;
  totalBytes?: number | null;
  percent?: number | null;
  bytesPerSecond: number;
  source: DownloadSourceUsed;
}

export type UpdateInstallPrepareReportStatus = "ready" | "failed";

export interface UpdateInstallPrepareRequest {
  requestId: string;
}
