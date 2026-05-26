import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, test, vi } from "vitest";
import { UpdateSettingsSection } from "./UpdateSettingsSection";
import type { UpdateSettings, UpdateState } from "./types";

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
  test("renders update infrastructure settings", () => {
    const markup = renderToStaticMarkup(
      <UpdateSettingsSection initialSettings={settings} initialStatus={status} />,
    );

    expect(markup).toContain("更新");
    expect(markup).toContain("当前版本：1.0.4");
    expect(markup).toContain("检查更新");
    expect(markup).toContain("启动后自动检查更新");
    expect(markup).toContain("有新版本时自动下载");
    expect(markup).toContain("下载源");
    expect(markup).toContain("Mirror 优先");
    expect(markup).toContain("GitHub 优先");
    expect(markup).toContain("Mirror 酱");
    expect(markup).toContain("已设置");
    expect(markup).toContain("清除 CDK");
    expect(markup).toContain("允许预发布版本");
    expect(markup).toContain("待更新版本：1.0.5");
    expect(markup).toContain("下载更新");
  });

  test("renders install schedule details after helper validation", () => {
    const scheduledStatus: UpdateState = {
      ...status,
      status: "installScheduled",
      assetPath: "/tmp/floral-notepaper_1.0.5_app.zip",
      downloadedAt: "2026-05-26T12:05:00Z",
      installLogPath: "/tmp/install-1.0.5.log",
      installMode: "dryRun",
      installStartedAt: "2026-05-26T12:06:00Z",
      installScheduledAt: "2026-05-26T12:06:01Z",
    };
    const markup = renderToStaticMarkup(
      <UpdateSettingsSection initialSettings={settings} initialStatus={scheduledStatus} />,
    );

    expect(markup).toContain("重新验证安装");
    expect(markup).toContain("dry-run 校验");
    expect(markup).toContain("/tmp/install-1.0.5.log");
  });
});
