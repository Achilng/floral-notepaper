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
          noteSurfaceAutoSave: true,
        }}
      />,
    );

    expect(markup).toContain("应用设置");
    expect(markup).toContain("D:\\Notes\\花笺");
    expect(markup).toContain("保存设置");
  });
});
