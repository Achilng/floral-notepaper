import { openUrl } from "@tauri-apps/plugin-opener";
import packageJson from "../../../package.json";

const GITHUB_RELEASES_API = "https://api.github.com/repos/Achilng/floral-notepaper/releases/latest";
const GITHUB_RELEASES_URL = "https://github.com/Achilng/floral-notepaper/releases/latest";

export type UpdateStatus = "idle" | "checking" | "available" | "current" | "error";

export interface UpdateInfo {
  currentVersion: string;
  latestVersion: string;
  releaseUrl: string;
  assetUrl: string;
  releaseName: string;
  publishedAt: string;
  body: string;
}

interface GitHubReleaseAsset {
  name?: unknown;
  browser_download_url?: unknown;
}

interface GitHubRelease {
  tag_name?: unknown;
  name?: unknown;
  html_url?: unknown;
  published_at?: unknown;
  body?: unknown;
  assets?: unknown;
}

export const CURRENT_VERSION = packageJson.version;

export async function checkForGitHubUpdate(): Promise<UpdateInfo | null> {
  const response = await fetch(GITHUB_RELEASES_API, {
    headers: {
      Accept: "application/vnd.github+json",
    },
  });

  if (!response.ok) {
    throw new Error(`GitHub release check failed: ${response.status}`);
  }

  const release = (await response.json()) as GitHubRelease;
  const latestVersion = normalizeVersion(String(release.tag_name ?? ""));
  if (!latestVersion || compareVersions(latestVersion, CURRENT_VERSION) <= 0) {
    return null;
  }

  const assets = Array.isArray(release.assets) ? (release.assets as GitHubReleaseAsset[]) : [];
  const assetUrl = chooseReleaseAssetUrl(assets);

  return {
    currentVersion: CURRENT_VERSION,
    latestVersion,
    releaseUrl: String(release.html_url || GITHUB_RELEASES_URL),
    assetUrl: assetUrl || String(release.html_url || GITHUB_RELEASES_URL),
    releaseName: String(release.name || release.tag_name || latestVersion),
    publishedAt: String(release.published_at || ""),
    body: String(release.body || ""),
  };
}

export function openUpdateDownload(info: UpdateInfo | null): Promise<void> {
  return openUrl(info?.assetUrl || GITHUB_RELEASES_URL);
}

function chooseReleaseAssetUrl(assets: GitHubReleaseAsset[]): string {
  const platform = navigator.platform.toLowerCase();
  const isWindows = platform.includes("win");
  const isMac = platform.includes("mac");

  const candidates = assets
    .map((asset) => ({
      name: String(asset.name || "").toLowerCase(),
      url: String(asset.browser_download_url || ""),
    }))
    .filter((asset) => asset.url);

  const preferred = candidates.find((asset) => {
    if (isWindows) {
      return asset.name.endsWith(".exe") || asset.name.endsWith(".msi") || asset.name.includes("setup");
    }

    if (isMac) {
      return asset.name.endsWith(".dmg") || asset.name.endsWith(".app.tar.gz");
    }

    return asset.name.endsWith(".appimage") || asset.name.endsWith(".deb") || asset.name.endsWith(".rpm");
  });

  return preferred?.url ?? candidates[0]?.url ?? "";
}

function normalizeVersion(value: string): string {
  return value.trim().replace(/^v/i, "");
}

function compareVersions(left: string, right: string): number {
  const leftParts = normalizeVersion(left).split(".").map(versionPartValue);
  const rightParts = normalizeVersion(right).split(".").map(versionPartValue);
  const length = Math.max(leftParts.length, rightParts.length);

  for (let index = 0; index < length; index += 1) {
    const diff = (leftParts[index] ?? 0) - (rightParts[index] ?? 0);
    if (diff !== 0) return diff;
  }

  return 0;
}

function versionPartValue(part: string): number {
  const match = /^\d+/.exec(part);
  return match ? Number(match[0]) : 0;
}
