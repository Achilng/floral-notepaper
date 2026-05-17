import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, test, vi } from "vitest";
import { MainWindow, runEditorUndo } from "./MainWindow";
import { LanguageContext } from "../features/i18n/LanguageContext";
import { translate } from "../features/i18n/LanguageContext";

describe("MainWindow settings", () => {
  test("can render the settings panel with the loaded config", () => {
    const config = {
      notesDir: "D:\\Notes\\花笺",
      globalShortcut: "Ctrl+Space",
      closeToTray: true,
      autostart: false,
      defaultViewMode: "split",
      noteAutoSave: true,
      noteSurfaceAutoSave: true,
      tileColor: "#f6f3ec",
      tileColorMode: "system" as const,
      theme: "light" as const,
      fontSize: 14,
      surfaceFontSize: 14,
      language: "zh-CN" as const,
    };

    const markup = renderToStaticMarkup(
      <LanguageContext.Provider
        value={{
          language: "zh-CN",
          setLanguage: () => {},
          t: (key, params) => translate("zh-CN", key, params),
        }}
      >
        <MainWindow initialSettingsOpen initialConfig={config} />
      </LanguageContext.Provider>,
    );

    expect(markup).toContain("应用设置");
    expect(markup).toContain("D:\\Notes\\花笺");
  });

  test("keeps draggable window chrome on the default arrow cursor", () => {
    const markup = renderToStaticMarkup(
      <LanguageContext.Provider
        value={{
          language: "zh-CN",
          setLanguage: () => {},
          t: (key, params) => translate("zh-CN", key, params),
        }}
      >
        <MainWindow />
      </LanguageContext.Provider>,
    );

    expect(markup).toContain("cursor-default");
    expect(markup).not.toContain("cursor-grab");
    expect(markup).not.toContain("cursor-grabbing");
  });
});

describe("MainWindow editor undo", () => {
  test("renders undo as an icon before save in the editor action bar", () => {
    const markup = renderToStaticMarkup(
      <LanguageContext.Provider
        value={{
          language: "zh-CN",
          setLanguage: () => {},
          t: (key, params) => translate("zh-CN", key, params),
        }}
      >
        <MainWindow />
      </LanguageContext.Provider>,
    );

    expect(markup).toContain('aria-label="撤销（Ctrl+Z）"');
    expect(markup).toContain('data-testid="main-editor-undo-icon"');
    expect(markup).not.toContain(">撤销<");
    expect(markup.indexOf('aria-label="撤销（Ctrl+Z）"')).toBeLessThan(markup.indexOf(">保存<"));
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
