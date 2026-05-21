import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, test } from "vitest";
import { MainWindow } from "./MainWindow";

describe("MainWindow settings", () => {
  test("can render the settings panel with the loaded config", () => {
    const markup = renderToStaticMarkup(
      <MainWindow
        initialSettingsOpen
        initialConfig={{
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

    expect(markup).not.toContain("cursor-grab");
    expect(markup).not.toContain("cursor-grabbing");
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
});
