import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, test, vi } from "vitest";
import { MainWindow, runEditorUndo } from "./MainWindow";

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

  test("renders the import Markdown icon as a down arrow", () => {
    const markup = renderToStaticMarkup(<MainWindow />);

    expect(markup).toContain('d="M12 3v12"');
    expect(markup).toContain('d="m7 10 5 5 5-5"');
    expect(markup).toContain('d="M5 21h14"');
    expect(markup).not.toContain('d="m7 14 5-5 5 5"');
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
