import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, test, vi } from "vitest";
import { SettingsPanel } from "./SettingsPanel";

const config = {
  notesDir: "D:\\Notes\\花笺",
  globalShortcut: "Ctrl+Space",
  closeToTray: true,
  autostart: false,
  defaultViewMode: "split" as const,
};

describe("SettingsPanel", () => {
  test("renders the core configurable app settings", () => {
    const markup = renderToStaticMarkup(
      <SettingsPanel
        config={config}
        isSaving={false}
        onChange={vi.fn()}
        onChooseNotesDir={vi.fn()}
        onClose={vi.fn()}
        onSave={vi.fn()}
      />,
    );

    expect(markup).toContain("应用设置");
    expect(markup).toContain("D:\\Notes\\花笺");
    expect(markup).toContain("选择文件夹");
    expect(markup).toContain("Ctrl+Space");
    expect(markup).toContain("Alt+Space");
    expect(markup).toContain("关闭到托盘");
    expect(markup).toContain("开机自启");
    expect(markup).toContain("默认视图");
    expect(markup).toContain("编辑");
    expect(markup).toContain("分栏");
    expect(markup).toContain("预览");
    expect(markup).toContain("保存设置");
  });
});
