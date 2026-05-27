import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, test, vi } from "vitest";
import { AboutPanel } from "./AboutPanel";

vi.mock("@tauri-apps/plugin-opener", () => ({
  openUrl: vi.fn(),
}));

vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn(),
}));

describe("AboutPanel", () => {
  test("renders app identity and update controls", () => {
    const markup = renderToStaticMarkup(<AboutPanel onClose={vi.fn()} />);

    expect(markup).toContain("关于");
    expect(markup).toContain("花笺");
    expect(markup).toContain("轻量、优雅、现代化的本地便签工具");
    expect(markup).toContain("更新");
    expect(markup).toContain("检查更新");
  });
});
