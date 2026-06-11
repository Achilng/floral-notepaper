import i18n from "../../locales";
import { describe, expect, test } from "vitest";
import { getUpdateErrorCode, getUpdateErrorMessage } from "./updateErrors";

describe("update error helpers", () => {
  test("reads error codes from nested invoke payloads", () => {
    const error = {
      payload: {
        code: "updateDownloadCancelled",
        message: "cancelled",
      },
    };

    expect(getUpdateErrorCode(error)).toBe("updateDownloadCancelled");
    expect(getUpdateErrorMessage(error, i18n.t.bind(i18n))).toBe("下载已取消");
  });

  test("maps backend-only update codes instead of returning raw rust messages", () => {
    const error = {
      code: "updateManifestUnsupportedSchema",
      message: "更新清单 schemaVersion 暂不受支持",
    };

    expect(getUpdateErrorMessage(error, i18n.t.bind(i18n))).toBe("更新清单格式版本暂不受支持");
  });

  test("maps helper cleanup failures through locale keys", () => {
    const error = {
      code: "updateInstallCleanupFailed",
      message: "安装后清理临时文件失败",
    };

    expect(getUpdateErrorMessage(error, i18n.t.bind(i18n))).toBe("安装后清理临时文件失败");
  });
});
