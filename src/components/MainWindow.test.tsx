import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, test, vi } from "vitest";
import { MainWindow, pinTileButtonTitle, runEditorUndo } from "./MainWindow";
import type { UpdateState } from "../features/update/types";

const pendingUpdateStatus: UpdateState = {
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

describe("MainWindow settings", () => {
  test("can render the settings panel with the loaded config", () => {
    const markup = renderToStaticMarkup(
      <MainWindow
        initialSettingsOpen
        initialConfig={{
          locale: "zh-CN",
          notesDir: "D:\\Notes\\花笺",
          globalShortcut: "Ctrl+Space",
          closeToTray: true,
          autostart: false,
          defaultViewMode: "split",
          noteAutoSave: true,
          noteSurfaceAutoSave: true,
          tileColor: "#f6f3ec",
          tileColorMode: "system",
          theme: "light",
          fontSize: 14,
          surfaceFontSize: 14,
          externalFileAutoSave: true,
          rememberSurfaceSize: true,
          tileCtrlClose: true,
          toggleVisibilityShortcut: "",
          tileRenderMarkdown: false,
        }}
      />,
    );

    expect(markup).toContain("应用设置");
    expect(markup).toContain("D:\\Notes\\花笺");
  });

  test("keeps draggable window chrome on the default arrow cursor", () => {
    const markup = renderToStaticMarkup(<MainWindow />);

    expect(markup).toContain("cursor-default");
    expect(markup).not.toContain("cursor-grab");
    expect(markup).not.toContain("cursor-grabbing");
  });

  test("renders shortcut registration failures in the title bar", () => {
    const markup = renderToStaticMarkup(
      <MainWindow initialErrorMessage="快捷键 Command+Option+N 注册失败" />,
    );

    expect(markup).toContain("快捷键 Command+Option+N 注册失败");
  });

  test("renders the import Markdown icon as a down arrow", () => {
    const markup = renderToStaticMarkup(<MainWindow />);

    expect(markup).toContain('d="M12 3v12"');
    expect(markup).toContain('d="m7 10 5 5 5-5"');
    expect(markup).toContain('d="M5 21h14"');
    expect(markup).not.toContain('d="m7 14 5-5 5 5"');
    expect(markup).not.toContain('d="m7 8 5-5 5 5"');
  });

  test("uses the body font for the main Markdown editor text", () => {
    const markup = renderToStaticMarkup(<MainWindow />);
    const editorMatch = markup.match(/<textarea[^>]*>/);

    expect(editorMatch?.[0]).toContain("font-body");
    expect(editorMatch?.[0]).not.toContain("font-mono");
  });

  test("shows the update icon without the expanded label for an existing pending update", () => {
    const markup = renderToStaticMarkup(<MainWindow initialUpdateStatus={pendingUpdateStatus} />);

    expect(markup).toContain('data-testid="main-about-update-icon"');
    expect(markup).not.toContain('data-testid="main-about-info-icon"');
    expect(markup).toContain('data-testid="main-about-update-label"');
    expect(markup).toContain("w-10 gap-0 px-0");
    expect(markup).toContain("max-w-0 translate-x-1 opacity-0");
  });

  test("can render the expanded about update reminder label", () => {
    const markup = renderToStaticMarkup(
      <MainWindow initialUpdateStatus={pendingUpdateStatus} initialAboutUpdateLabelVisible />,
    );

    expect(markup).toContain('data-testid="main-about-update-icon"');
    expect(markup).toContain('data-testid="main-about-update-label"');
    expect(markup).toContain("w-[72px] gap-1.5 px-3");
    expect(markup).toContain("max-w-[24px] translate-x-0 opacity-100");
  });

  test("labels the pin button as a toggle", () => {
    expect(pinTileButtonTitle(false)).toBe("钉到屏幕");
    expect(pinTileButtonTitle(true)).toBe("取消钉屏");
  });
});

describe("MainWindow editor undo", () => {
  test("renders undo as an icon before save in the editor action bar", () => {
    const markup = renderToStaticMarkup(<MainWindow />);

    expect(markup).toContain('aria-label="撤销"');
    expect(markup).toContain('data-testid="main-editor-undo-icon"');
    expect(markup).not.toContain(">撤销<");
    expect(markup.indexOf('aria-label="撤销"')).toBeLessThan(markup.indexOf(">保存<"));
  });

  test("focuses the editor and runs the browser undo command", () => {
    const focus = vi.fn();
    const execCommand = vi.fn(() => true);
    const textarea = { disabled: false, focus } as unknown as HTMLTextAreaElement;

    const undone = runEditorUndo(textarea, { execCommand });

    expect(undone).toBe(true);
    expect(focus).toHaveBeenCalledOnce();
    expect(execCommand).toHaveBeenCalledWith("undo");
  });
});
