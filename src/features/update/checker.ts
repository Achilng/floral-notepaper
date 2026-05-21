import { getVersion } from "@tauri-apps/api/app";
import { confirm, message } from "@tauri-apps/plugin-dialog";
import { openPath, openUrl } from "@tauri-apps/plugin-opener";
import { downloadUpdateAsset } from "./api";

export const RELEASES_API_URL =
  "https://api.github.com/repos/Achilng/floral-notepaper/releases/latest";
export const RELEASES_PAGE_URL =
  "https://github.com/Achilng/floral-notepaper/releases/latest";

export interface LatestReleaseInfo {
  version: string;
  tagName: string;
  url: string;
  assetName: string | null;
  assetUrl: string | null;
}

interface LatestReleasePayload {
  tag_name?: unknown;
  html_url?: unknown;
  assets?: unknown;
}

interface ReleaseAssetPayload {
  name?: unknown;
  browser_download_url?: unknown;
}

interface UpdateCheckDeps {
  getCurrentVersion: () => Promise<string>;
  fetchRelease: typeof fetch;
  confirmOpen: typeof confirm;
  showMessage: typeof message;
  openReleasePage: (url: string) => Promise<void>;
  launchInstaller: (path: string) => Promise<void>;
  downloadAsset: (url: string, fileName: string) => Promise<string>;
  platform: string;
}

const defaultDeps: UpdateCheckDeps = {
  getCurrentVersion: getVersion,
  fetchRelease: fetch,
  confirmOpen: confirm,
  showMessage: message,
  openReleasePage: (url) => openUrl(url),
  launchInstaller: (path) => openPath(path),
  downloadAsset: downloadUpdateAsset,
  platform: detectPlatform(),
};

function detectPlatform(): string {
  const userAgent =
    typeof navigator === "undefined" ? "" : navigator.userAgent.toLowerCase();
  if (userAgent.includes("windows")) return "windows";
  if (userAgent.includes("mac")) return "macos";
  return "unknown";
}

export function normalizeVersion(value: string): string {
  return value.trim().replace(/^[vV]/, "").split("-")[0] ?? "";
}

function parseVersionParts(value: string): number[] | null {
  const normalized = normalizeVersion(value);
  if (!normalized) return null;

  const parts = normalized.split(".");
  if (parts.some((part) => !/^\d+$/.test(part))) return null;
  return parts.map(Number);
}

export function compareVersions(left: string, right: string): number {
  const leftParts = parseVersionParts(left);
  const rightParts = parseVersionParts(right);
  if (!leftParts || !rightParts) return 0;

  const maxLength = Math.max(leftParts.length, rightParts.length);
  for (let index = 0; index < maxLength; index += 1) {
    const leftValue = leftParts[index] ?? 0;
    const rightValue = rightParts[index] ?? 0;
    if (leftValue > rightValue) return 1;
    if (leftValue < rightValue) return -1;
  }
  return 0;
}

function isReleaseAssetPayload(value: unknown): value is ReleaseAssetPayload {
  return !!value && typeof value === "object";
}

export function selectPreferredAsset(
  assets: unknown,
  platform: string,
): { name: string; url: string } | null {
  if (!Array.isArray(assets)) return null;

  const parsedAssets = assets
    .filter(isReleaseAssetPayload)
    .map((asset) => ({
      name: typeof asset.name === "string" ? asset.name : "",
      url:
        typeof asset.browser_download_url === "string"
          ? asset.browser_download_url
          : "",
    }))
    .filter((asset) => asset.name && asset.url);

  if (platform === "windows") {
    return (
      parsedAssets.find((asset) => /_x64-setup\.exe$/i.test(asset.name)) ?? null
    );
  }

  if (platform === "macos") {
    return parsedAssets.find((asset) => /\.dmg$/i.test(asset.name)) ?? null;
  }

  return null;
}

export function extractLatestReleaseInfo(
  payload: LatestReleasePayload,
  platform = "unknown",
): LatestReleaseInfo | null {
  if (typeof payload.tag_name !== "string" || !payload.tag_name.trim()) {
    return null;
  }

  const version = normalizeVersion(payload.tag_name);
  if (!parseVersionParts(version)) return null;
  const asset = selectPreferredAsset(payload.assets, platform);

  return {
    version,
    tagName: payload.tag_name,
    url:
      typeof payload.html_url === "string" && payload.html_url.trim()
        ? payload.html_url
        : RELEASES_PAGE_URL,
    assetName: asset?.name ?? null,
    assetUrl: asset?.url ?? null,
  };
}

export async function checkForAppUpdate(
  deps: Partial<UpdateCheckDeps> = {},
): Promise<void> {
  const resolvedDeps = { ...defaultDeps, ...deps };
  try {
    const currentVersion = normalizeVersion(
      await resolvedDeps.getCurrentVersion(),
    );
    if (!parseVersionParts(currentVersion)) return;

    const response = await resolvedDeps.fetchRelease(RELEASES_API_URL, {
      headers: {
        Accept: "application/vnd.github+json",
      },
    });
    if (!response.ok) return;

    const releaseInfo = extractLatestReleaseInfo(
      (await response.json()) as LatestReleasePayload,
      resolvedDeps.platform,
    );
    if (!releaseInfo) return;

    if (compareVersions(releaseInfo.version, currentVersion) <= 0) return;

    const shouldOpen = await resolvedDeps.confirmOpen(
      `发现新版本 v${releaseInfo.version}，当前版本为 v${currentVersion}。是否立即下载安装更新？`,
      {
        title: "发现更新",
        kind: "info",
      },
    );
    if (!shouldOpen) return;

    if (releaseInfo.assetUrl && releaseInfo.assetName) {
      try {
        const installerPath = await resolvedDeps.downloadAsset(
          releaseInfo.assetUrl,
          releaseInfo.assetName,
        );
        await resolvedDeps.showMessage("更新安装包已下载完成，即将启动安装程序。", {
          title: "准备安装更新",
          kind: "info",
        });
        await resolvedDeps.launchInstaller(installerPath);
        return;
      } catch {
        await resolvedDeps.showMessage(
          "自动下载安装失败，将为你打开官方发布页手动下载。",
          {
            title: "下载失败",
            kind: "warning",
          },
        );
      }
    }

    await resolvedDeps.openReleasePage(releaseInfo.url);
  } catch {
    // Silently ignore transient update-check failures.
  }
}
