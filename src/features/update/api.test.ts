import { invoke } from "@tauri-apps/api/core";
import { beforeEach, describe, expect, test, vi } from "vitest";
import {
  cancelUpdate,
  checkForUpdates,
  clearMirrorChyanCdk,
  downloadUpdate,
  getUpdateSettings,
  getUpdateStatus,
  installUpdate,
  reportInstallPreparation,
  saveUpdateSettings,
  setMirrorChyanCdk,
} from "./api";
import type { UpdateSettings } from "./types";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

const mockedInvoke = vi.mocked(invoke);

describe("update api", () => {
  beforeEach(() => {
    mockedInvoke.mockReset();
  });

  test("gets update status", async () => {
    const status = {
      status: "idle",
      currentVersion: "1.0.4",
      latestVersion: null,
      channel: "stable",
      assetName: null,
      assetPath: null,
      assetSha256: null,
      assetSize: null,
      source: null,
      checkedAt: null,
      downloadedAt: null,
      installLogPath: null,
      installMode: null,
      installStartedAt: null,
      installScheduledAt: null,
      lastError: null,
    };
    mockedInvoke.mockResolvedValue(status);

    await expect(getUpdateStatus()).resolves.toBe(status);

    expect(invoke).toHaveBeenCalledWith("update_status");
  });

  test("gets and saves update settings", async () => {
    const settings: UpdateSettings = {
      autoCheck: true,
      autoDownload: false,
      checkIntervalHours: 24,
      checkSourcePreference: "githubFirst",
      downloadSourcePreference: "mirrorChyanFirst",
      channel: "stable",
      allowPrerelease: false,
      lastAutoCheckAt: null,
      hasMirrorChyanCdk: false,
    };
    mockedInvoke.mockResolvedValue(settings);

    await expect(getUpdateSettings()).resolves.toBe(settings);
    await expect(saveUpdateSettings(settings)).resolves.toBe(settings);

    expect(invoke).toHaveBeenNthCalledWith(1, "update_settings_get");
    expect(invoke).toHaveBeenNthCalledWith(2, "update_settings_save", { settings });
  });

  test("sets and clears Mirror CDK", async () => {
    mockedInvoke.mockResolvedValue(undefined);

    await expect(setMirrorChyanCdk("secret-cdk")).resolves.toBeUndefined();
    await expect(clearMirrorChyanCdk()).resolves.toBeUndefined();

    expect(invoke).toHaveBeenNthCalledWith(1, "update_mirror_chyan_cdk_set", { cdk: "secret-cdk" });
    expect(invoke).toHaveBeenNthCalledWith(2, "update_mirror_chyan_cdk_clear");
  });

  test("keeps final command names for staged operations", async () => {
    mockedInvoke.mockResolvedValue({
      status: "installing",
      logPath: "/tmp/install.log",
      mode: "apply",
    });

    await checkForUpdates(true).catch(() => undefined);
    await downloadUpdate("github").catch(() => undefined);
    await installUpdate().catch(() => undefined);
    await cancelUpdate().catch(() => undefined);

    expect(invoke).toHaveBeenNthCalledWith(1, "update_check", { manual: true });
    expect(invoke).toHaveBeenNthCalledWith(2, "update_download", { source: "github" });
    expect(invoke).toHaveBeenNthCalledWith(3, "update_install");
    expect(invoke).toHaveBeenNthCalledWith(4, "update_cancel");
  });

  test("reports install preparation results with the final command name", async () => {
    mockedInvoke.mockResolvedValue(undefined);

    await expect(
      reportInstallPreparation("request-1", "main", "failed", "save failed"),
    ).resolves.toBeUndefined();

    expect(invoke).toHaveBeenCalledWith("update_install_prepare_report", {
      requestId: "request-1",
      windowLabel: "main",
      status: "failed",
      message: "save failed",
    });
  });
});
