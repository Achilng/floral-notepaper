import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, test, vi } from "vitest";
import { AboutPanel } from "./AboutPanel";

vi.mock("@tauri-apps/api/app", () => ({
  getVersion: vi.fn(() => Promise.resolve("1.0.4")),
}));

vi.mock("@tauri-apps/plugin-opener", () => ({
  openUrl: vi.fn(),
}));

vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn(),
}));

vi.mock("../generated/contributors.json", () => ({
  default: [
    {
      login: "TestUser",
      avatar_url: "https://avatars.githubusercontent.com/u/1?v=4",
      html_url: "https://github.com/TestUser",
    },
  ],
}));

describe("AboutPanel", () => {
  test("renders app identity and update section placeholder", () => {
    const markup = renderToStaticMarkup(<AboutPanel onClose={vi.fn()} />);

    expect(markup).toContain("关于");
    expect(markup).toContain("花笺");
    expect(markup).toContain("轻量、优雅、现代化的本地便签工具");
    expect(markup).toContain("更新");
    // Update status has not hydrated yet: the update section shows a loading
    // placeholder instead of full update controls (avoids MSIX controls flash).
    expect(markup).toContain("正在读取更新设置");
    expect(markup).not.toContain("检查更新");
    expect(markup).not.toContain("自动检查更新");
  });

  test("renders github link, feedback link, and contributors", () => {
    const markup = renderToStaticMarkup(<AboutPanel onClose={vi.fn()} />);

    expect(markup).toContain("GitHub");
    expect(markup).toContain("反馈问题");
    expect(markup).toContain("许可证");
    expect(markup).toContain("贡献者");
    expect(markup).toContain("TestUser");
  });
});
