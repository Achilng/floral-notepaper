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
});
