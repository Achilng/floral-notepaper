import { invoke } from "@tauri-apps/api/core";
import { beforeEach, describe, expect, test, vi } from "vitest";
import { saveFeishuSettings, syncAllNotesToFeishu } from "./api";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

describe("Feishu API bridge", () => {
  beforeEach(() => {
    vi.mocked(invoke).mockReset();
  });

  test("sends settings and a new secret to the secure backend command", async () => {
    const settings = {
      enabled: true,
      appId: "cli_test",
      folderToken: "fld_test",
      hasSecret: false,
      connected: false,
    };
    vi.mocked(invoke).mockResolvedValueOnce({ ...settings, hasSecret: true });

    await saveFeishuSettings(settings, "secret");

    expect(invoke).toHaveBeenCalledWith("feishu_settings_save", {
      settings,
      appSecret: "secret",
    });
  });

  test("uses the dedicated full-sync command", async () => {
    vi.mocked(invoke).mockResolvedValueOnce({ total: 1, synced: 1, skipped: 0, failed: 0 });

    await syncAllNotesToFeishu();

    expect(invoke).toHaveBeenCalledWith("feishu_sync_all");
  });
});
