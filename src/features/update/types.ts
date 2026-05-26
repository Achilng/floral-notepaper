export type DownloadSourcePreference = "mirrorFirst" | "githubFirst" | "mirrorOnly" | "githubOnly";

export type DownloadSourceUsed = "mirror" | "github";

export type UpdateChannel = "stable" | "beta";

export type UpdateInstallMode = "dryRun" | "test";

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
  downloadSourcePreference: DownloadSourcePreference;
  channel: UpdateChannel;
  allowPrerelease: boolean;
  lastAutoCheckAt?: string | null;
  hasMirrorCdk: boolean;
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
  canDownloadFromMirror: boolean;
  canDownloadFromGithub: boolean;
  recommendedSource?: DownloadSourceUsed | null;
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
