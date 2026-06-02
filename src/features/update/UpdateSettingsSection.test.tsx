import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, test, vi } from "vitest";
import { deriveDownloadProgressState, UpdateSettingsSection } from "./UpdateSettingsSection";
import type { UpdateSettings, UpdateState } from "./types";

vi.mock("@tauri-apps/plugin-dialog", () => ({
  message: vi.fn(),
}));

vi.mock("@tauri-apps/plugin-opener", () => ({
  openUrl: vi.fn(),
}));

vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn(),
}));

const settings: UpdateSettings = {
  autoCheck: true,
  autoDownload: false,
  checkIntervalHours: 24,
  checkSourcePreference: "githubFirst",
  downloadSourcePreference: "mirrorFirst",
  channel: "stable",
  allowPrerelease: false,
  lastAutoCheckAt: null,
  hasMirrorCdk: true,
};

const status: UpdateState = {
  status: "available",
  currentVersion: "1.0.4",
  latestVersion: "1.0.5",
  channel: "stable",
  assetName: "floral-notepaper_1.0.5_macos_aarch64_app.zip",
  assetPath: null,
  assetSha256: "abc",
  assetSize: 12345678,
  source: "github",
  checkedAt: "2026-05-26T12:00:00Z",
  downloadedAt: null,
  installLogPath: null,
  installMode: null,
  installStartedAt: null,
  installScheduledAt: null,
  lastError: null,
};

describe("UpdateSettingsSection", () => {
  test("renders check-only update controls for the about panel", () => {
    const markup = renderToStaticMarkup(
      <UpdateSettingsSection initialSettings={settings} initialStatus={status} mode="checkOnly" />,
    );

    expect(markup).toContain("更新");
    expect(markup).toContain("当前版本：1.0.4");
    expect(markup).toContain("检查更新");
    expect(markup).toContain("待更新版本：1.0.5");
    expect(markup).toContain("下载更新");
    expect(markup).not.toContain("自动检查更新");
    expect(markup).not.toContain("Mirror酱");
  });

  test("renders update settings for the settings panel", () => {
    const markup = renderToStaticMarkup(
      <UpdateSettingsSection
        initialSettings={settings}
        initialStatus={status}
        mode="settingsOnly"
      />,
    );

    expect(markup).toContain("更新设置");
    expect(markup).toContain("自动检查更新");
    expect(markup).toContain("有新版本时自动下载");
    expect(markup).toContain("下载源");
    expect(markup).toContain("GitHub");
    expect(markup).toContain("Mirror酱");
    expect(markup).toContain("Mirror酱");
    expect(markup).toContain("已设置");
    expect(markup).toContain("清除 CDK");
    expect(markup).toContain("允许预发布版本");
    expect(markup).not.toContain("当前版本：1.0.4");
    expect(markup).not.toContain("待更新版本：1.0.5");
  });

  test("preserves non-standard check interval values in the settings UI", () => {
    const markup = renderToStaticMarkup(
      <UpdateSettingsSection
        initialSettings={{ ...settings, checkIntervalHours: 12 }}
        initialStatus={status}
        mode="settingsOnly"
      />,
    );

    expect(markup).toContain("12 小时");
  });

  test("renders install in-progress details after launching the helper", () => {
    const scheduledStatus: UpdateState = {
      ...status,
      status: "installing",
      assetPath: "/tmp/floral-notepaper_1.0.5_app.zip",
      downloadedAt: "2026-05-26T12:05:00Z",
      installLogPath: "/tmp/install-1.0.5.log",
      installMode: "apply",
      installStartedAt: "2026-05-26T12:06:00Z",
      installScheduledAt: null,
    };
    const markup = renderToStaticMarkup(
      <UpdateSettingsSection
        initialSettings={settings}
        initialStatus={scheduledStatus}
        mode="checkOnly"
      />,
    );

    expect(markup).toContain("正在准备退出应用并安装更新");
    expect(markup).not.toContain("dry-run 校验");
    expect(markup).toContain("/tmp/install-1.0.5.log");
  });

  test("shows re-download instead of install when the last error requires a fresh asset", () => {
    const failedStatus: UpdateState = {
      ...status,
      status: "failed",
      assetPath: "/tmp/floral-notepaper_1.0.5_app.zip",
      downloadedAt: "2026-05-26T12:05:00Z",
      lastError: {
        code: "updateInstallInterrupted",
        message: "asset missing",
        recoverable: true,
        action: "retryDownload",
      },
    };

    const markup = renderToStaticMarkup(
      <UpdateSettingsSection
        initialSettings={settings}
        initialStatus={failedStatus}
        mode="checkOnly"
      />,
    );

    expect(markup).toContain("下载更新");
    expect(markup).not.toContain("安装并重启");
    expect(markup).not.toContain("重新尝试安装");
  });

  test("renders the last install failure notice from initial status", () => {
    const failedStatus: UpdateState = {
      ...status,
      status: "failed",
      assetPath: "/tmp/floral-notepaper_1.0.5_app.zip",
      downloadedAt: "2026-05-26T12:05:00Z",
      installLogPath: "/tmp/install-1.0.5.log",
      lastError: {
        code: "updateInstallVersionMismatch",
        message: "still on old version",
        recoverable: true,
        action: "retryInstall",
      },
    };

    const markup = renderToStaticMarkup(
      <UpdateSettingsSection
        initialSettings={settings}
        initialStatus={failedStatus}
        mode="checkOnly"
      />,
    );

    expect(markup).toContain("安装后重新打开的仍是旧版本，请直接重试安装");
    expect(markup).toContain("/tmp/install-1.0.5.log");
  });

  test("uses the latest channel when deriving optimistic download state without prior status", () => {
    const nextStatus = deriveDownloadProgressState(
      null,
      {
        version: "1.0.6",
        assetName: "floral-notepaper_1.0.6_macos_aarch64_app.zip",
        downloadedBytes: 1024,
        totalBytes: 2048,
        percent: 50,
        bytesPerSecond: 1024,
        source: "mirror",
      },
      "beta",
    );

    expect(nextStatus.channel).toBe("beta");
    expect(nextStatus.status).toBe("downloading");
    expect(nextStatus.source).toBe("mirror");
    expect(nextStatus.assetSize).toBe(2048);
  });
});
