import { useEffect } from "react";
import "./App.css";
import { ContextMenuProvider } from "./components/ContextMenu";
import { MainWindow } from "./components/MainWindow";
import { NotePad } from "./components/NotePad";
import { TileShowcase } from "./components/TileShowcase";
import { getConfig } from "./features/settings/api";
import { applyCustomAccentColor, applyCustomTextColor } from "./features/settings/customColors";
import { applyTheme, watchSystemTheme } from "./features/settings/theme";
import type { AppConfig, ThemeOption } from "./features/settings/types";
import { UpdateBanner } from "./features/updater/UpdateBanner";
import { useAutoUpdate } from "./features/updater/useAutoUpdate";
import { getInitialRoute } from "./features/windows/windowRoutes";
import { listen } from "@tauri-apps/api/event";

function applyConfigColors(config: AppConfig) {
  applyCustomAccentColor(config.accentColorMode ?? "default", config.accentColor ?? "#2d5a3d");
  applyCustomTextColor(config.textColorMode ?? "default", config.textColor ?? "#1a1a18");
}

function App() {
  const route = getInitialRoute();
  const activeView = route.view;
  const { updateInfo, dismiss, openDownload } = useAutoUpdate();

  useEffect(() => {
    let cleanup = () => {};
    getConfig()
      .then((config) => {
        const theme = (config.theme || "system") as ThemeOption;
        applyTheme(theme);
        applyConfigColors(config);
        cleanup = watchSystemTheme(theme);
      })
      .catch(() => {});
    return () => cleanup();
  }, []);

  useEffect(() => {
    const unlisten = listen<AppConfig>("config-changed", (event) => {
      const config = event.payload;
      const theme = (config.theme || "system") as ThemeOption;
      applyTheme(theme);
      applyConfigColors(config);
      watchSystemTheme(theme);
    });
    return () => {
      void unlisten.then((fn) => fn());
    };
  }, []);

  useEffect(() => {
    const preventSystemMenu = (e: KeyboardEvent) => {
      if (e.altKey && e.code === "Space") {
        e.preventDefault();
      }
    };
    document.addEventListener("keydown", preventSystemMenu, true);
    return () =>
      document.removeEventListener("keydown", preventSystemMenu, true);
  }, []);

  return (
    <ContextMenuProvider>
      <div className="h-screen font-body text-ink overflow-hidden">
        {activeView === "main" ? (
          <MainWindow />
        ) : activeView === "notepad" ? (
          <NotePad initialNoteId={route.noteId} />
        ) : (
          <TileShowcase noteId={route.noteId} />
        )}
      </div>
      {updateInfo && (
        <UpdateBanner
          update={updateInfo}
          onDownload={openDownload}
          onDismiss={dismiss}
        />
      )}
    </ContextMenuProvider>
  );
}

export default App;
