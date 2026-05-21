import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, test, vi } from "vitest";
import { SettingsPanel } from "./SettingsPanel";

const config = {
  locale: "zh-CN",
  notesDir: "D:\\Notes\\花笺",
  globalShortcut: "Command+Option+N",
  closeToTray: true,
  autostart: false,
  defaultViewMode: "split" as const,
  noteAutoSave: true,
  noteSurfaceAutoSave: true,
  tileColor: "#f6f3ec",
  tileColorMode: "custom" as const,
  theme: "light" as const,
  fontSize: 14,
  surfaceFontSize: 14,
  externalFileAutoSave: true,
  rememberSurfaceSize: true,
  tileCtrlClose: true,
  toggleVisibilityShortcut: "",
  tileRenderMarkdown: false,
};

describe("SettingsPanel", () => {
  test("renders the core configurable app settings", () => {
    const markup = renderToStaticMarkup(
      <SettingsPanel
        config={config}
        onChange={vi.fn()}
        onChooseNotesDir={vi.fn()}
        onClose={vi.fn()}
      />,
    );

    expect(markup).toContain("应用设置");
    expect(markup).toContain("D:\\Notes\\花笺");
    expect(markup).toContain("选择文件夹");
    expect(markup).toContain("快捷记录快捷键");
    expect(markup).toContain("Command+Option+N");
    expect(markup).toContain("检测");
    expect(markup).toContain("关闭到托盘");
    expect(markup).toContain("开机自启");
    expect(markup).toContain("自动保存笔记");
    expect(markup).toContain("小窗笔记自动保存");
    expect(markup).toContain("磁贴颜色");
    expect(markup).toContain("跟随主题");
    expect(markup).toContain("自定义");
    expect(markup).toContain('type="color"');
    expect(markup).toContain('value="#f6f3ec"');
    expect(markup).toContain("默认视图");
    expect(markup).toContain("编辑");
    expect(markup).toContain("分栏");
    expect(markup).toContain("预览");
  });
});
