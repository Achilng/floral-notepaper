import i18n from "../../locales";
import {
  ABOUT_UPDATE_LABEL_DURATION_MS,
  applyAboutUpdateStatus,
  createAboutUpdateReminderState,
  dismissAboutUpdateReminderText,
  getInitialUpdateStatusNotice,
  getUpdateCheckCompletionNotice,
} from "./presentation";
import type { UpdateState } from "./types";
import { describe, expect, test } from "vitest";

const availableStatus: UpdateState = {
  status: "available",
  currentVersion: "1.0.3",
  latestVersion: "1.0.4",
  channel: "stable",
  assetName: "floral-notepaper_1.0.4_macos_aarch64_app.zip",
  assetPath: null,
  assetSha256: "abc",
  assetSize: 12345678,
  assetUrl: "https://example.com/download.zip",
  source: "github",
  checkedAt: "2026-05-27T08:00:00Z",
  downloadedAt: null,
  installLogPath: null,
  installMode: null,
  installStartedAt: null,
  installScheduledAt: null,
  lastError: null,
};

describe("update presentation helpers", () => {
  test("keeps the about reminder collapsed for initial pending updates", () => {
    expect(createAboutUpdateReminderState(availableStatus)).toEqual({
      hasPendingUpdate: true,
      showText: false,
    });
  });

  test("opens the about reminder when a new pending update arrives", () => {
    const next = applyAboutUpdateStatus(createAboutUpdateReminderState(null), availableStatus);

    expect(next).toEqual({
      hasPendingUpdate: true,
      showText: true,
    });
  });

  test("dismisses the expanded reminder text while keeping the update icon", () => {
    expect(
      dismissAboutUpdateReminderText({
        hasPendingUpdate: true,
        showText: true,
      }),
    ).toEqual({
      hasPendingUpdate: true,
      showText: false,
    });
  });

  test("uses a 30 second reminder window", () => {
    expect(ABOUT_UPDATE_LABEL_DURATION_MS).toBe(30_000);
  });

  test("builds checked notices for available, latest, and failed states", () => {
    expect(getUpdateCheckCompletionNotice(availableStatus, i18n.t.bind(i18n))).toEqual({
      tone: "success",
      text: "发现新版本 1.0.4",
    });

    expect(
      getUpdateCheckCompletionNotice(
        {
          ...availableStatus,
          status: "idle",
          latestVersion: null,
        },
        i18n.t.bind(i18n),
      ),
    ).toEqual({
      tone: "success",
      text: "当前已是最新版本",
    });

    expect(
      getUpdateCheckCompletionNotice(
        {
          ...availableStatus,
          status: "failed",
          latestVersion: null,
          lastError: {
            code: "updateGithubApi",
            message: "GitHub API 请求失败：请求超时",
            recoverable: true,
            action: "retry",
          },
        },
        i18n.t.bind(i18n),
      ),
    ).toEqual({
      tone: "error",
      text: "GitHub API 请求失败，请检查网络后重试",
    });
  });

  test("only surfaces failed status on initial render", () => {
    expect(getInitialUpdateStatusNotice(availableStatus, i18n.t.bind(i18n))).toBeNull();

    expect(
      getInitialUpdateStatusNotice(
        {
          ...availableStatus,
          status: "failed",
          lastError: {
            code: "updateInstallVersionMismatch",
            message: "still on old version",
            recoverable: true,
            action: "retryInstall",
          },
        },
        i18n.t.bind(i18n),
      ),
    ).toEqual({
      tone: "error",
      text: "安装后重新打开的仍是旧版本，请直接重试安装",
    });
  });
});
