import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, test, vi } from "vitest";
import { FeishuSettingsSection } from "./FeishuSettingsSection";

vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn(() => Promise.resolve(vi.fn())),
}));

vi.mock("@tauri-apps/plugin-opener", () => ({
  openUrl: vi.fn(),
}));

describe("FeishuSettingsSection", () => {
  test("renders credential, status, and one-way-sync guidance", () => {
    const markup = renderToStaticMarkup(<FeishuSettingsSection />);

    expect(markup).toContain("飞书云同步");
    expect(markup).toContain("App ID");
    expect(markup).toContain("App Secret");
    expect(markup).toContain("目标文件夹 Token");
    expect(markup).toContain("保存配置");
    expect(markup).toContain("测试连接");
    expect(markup).toContain("登录飞书");
    expect(markup).toContain("立即同步全部");
    expect(markup).toContain("单向同步");
    expect(markup).toContain("创建及编辑新版文档");
  });
});
