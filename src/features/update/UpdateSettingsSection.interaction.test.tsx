// @vitest-environment jsdom
import { cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { afterEach, beforeAll, beforeEach, describe, expect, test, vi } from "vitest";
import { UpdateSettingsSection } from "./UpdateSettingsSection";
import type { UpdateSettings, UpdateState } from "./types";

// `SlidingButtonGroup` schedules a highlight animation via requestAnimationFrame,
// which jsdom does not implement; polyfill it before any component renders.
beforeAll(() => {
  if (typeof globalThis.requestAnimationFrame === "undefined") {
    globalThis.requestAnimationFrame = (callback) =>
      setTimeout(() => callback(performance.now()), 0) as unknown as number;
    globalThis.cancelAnimationFrame = (handle) => clearTimeout(handle as unknown as number);
  }
});

const apiMocks = vi.hoisted(() => ({
  saveUpdateSettings: vi.fn(),
}));

vi.mock("./api", () => ({
  cancelUpdate: vi.fn(),
  checkForUpdates: vi.fn(),
  downloadUpdate: vi.fn(),
  getMirrorChyanCdk: vi.fn().mockResolvedValue(null),
  getUpdateSettings: vi.fn(),
  getUpdateStatus: vi.fn(),
  installUpdate: vi.fn(),
  saveUpdateSettings: apiMocks.saveUpdateSettings,
  setMirrorChyanCdk: vi.fn().mockResolvedValue(undefined),
}));

vi.mock("@tauri-apps/plugin-dialog", () => ({
  message: vi.fn(),
}));

vi.mock("@tauri-apps/plugin-opener", () => ({
  openUrl: vi.fn(),
}));

vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn().mockResolvedValue(() => {}),
}));

const settings: UpdateSettings = {
  autoCheck: true,
  autoDownload: false,
  checkIntervalHours: 24,
  checkSourcePreference: "githubFirst",
  downloadSourcePreference: "mirrorChyanFirst",
  channel: "stable",
  allowPrerelease: false,
  lastAutoCheckAt: null,
  hasMirrorChyanCdk: true,
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

describe("UpdateSettingsSection 交互回归", () => {
  // Vitest 未开启 `globals`，@testing-library/react 无法自动清理，需手动清理。
  afterEach(cleanup);

  beforeEach(() => {
    apiMocks.saveUpdateSettings.mockReset();
    apiMocks.saveUpdateSettings.mockImplementation(async (saved: UpdateSettings) => saved);
  });

  test("自动检查更新渲染为 role=switch 的按钮，而非隐藏 checkbox", () => {
    render(
      <UpdateSettingsSection
        initialSettings={settings}
        initialStatus={status}
        mode="settingsOnly"
      />,
    );

    const toggle = screen.getByRole("switch", { name: "自动检查更新" });
    expect(toggle.tagName).toBe("BUTTON");
    expect(toggle.getAttribute("type")).toBe("button");
    expect(toggle.getAttribute("aria-checked")).toBe("true");

    // 旧实现用 <label> + sr-only <input type="checkbox">；聚焦该隐藏输入会把
    // 可滚动的设置面板带偏（异常偏移）。切换行内部不应再存在任何 checkbox。
    expect(within(toggle).queryByRole("checkbox")).toBeNull();
    // 整个设置面板也不应有遗留的 checkbox 角色。
    expect(screen.queryAllByRole("checkbox")).toHaveLength(0);
  });

  test("两个开关按顺序渲染，自动检查更新在前", () => {
    render(
      <UpdateSettingsSection
        initialSettings={settings}
        initialStatus={status}
        mode="settingsOnly"
      />,
    );

    const switches = screen.getAllByRole("switch");
    expect(switches).toHaveLength(2);
    expect(switches[0].textContent).toContain("自动检查更新");
    expect(switches[1].textContent).toContain("有新版本时自动下载");
  });

  test("点击自动检查更新后，设置面板不消失、不变白、不偏移", async () => {
    render(
      <UpdateSettingsSection
        initialSettings={settings}
        initialStatus={status}
        mode="settingsOnly"
      />,
    );

    const toggle = screen.getByRole("switch", { name: "自动检查更新" });
    fireEvent.click(toggle);

    // 状态立即切换。
    expect(toggle.getAttribute("aria-checked")).toBe("false");

    // 等待异步保存完成，确保后续 setState 也经过 act 冲刷。
    await waitFor(() =>
      expect(apiMocks.saveUpdateSettings).toHaveBeenCalledWith(
        expect.objectContaining({ autoCheck: false }),
      ),
    );

    // 面板未消失：开关仍可访问。
    expect(screen.getByRole("switch", { name: "自动检查更新" })).toBeTruthy();

    // 面板未变白：其余设置内容仍在，未退回“正在读取…”占位。
    expect(screen.getByText("有新版本时自动下载")).toBeTruthy();
    expect(screen.getByText("检查频率")).toBeTruthy();
    expect(screen.getByText("下载源")).toBeTruthy();
    expect(screen.queryByText("正在读取更新设置...")).toBeNull();

    // 面板未偏移：两个开关仍按原顺序渲染。
    const switches = screen.getAllByRole("switch");
    expect(switches[0].textContent).toContain("自动检查更新");
    expect(switches[1].textContent).toContain("有新版本时自动下载");
  });
});
