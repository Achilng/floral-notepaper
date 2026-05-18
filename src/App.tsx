import { useEffect } from "react";
import "./App.css";
import { ContextMenuProvider } from "./components/ContextMenu";
import { MainWindow } from "./components/MainWindow";
import { NotePad } from "./components/NotePad";
import { TileShowcase } from "./components/TileShowcase";
import { getConfig } from "./features/settings/api";
import { applyAppConfig, watchSystemTheme } from "./features/settings/theme";
import type { AppConfig, ThemeOption } from "./features/settings/types";
import { getInitialRoute } from "./features/windows/windowRoutes";
import { listen } from "@tauri-apps/api/event";

function App() {
  const route = getInitialRoute();
  const activeView = route.view;

  useEffect(() => {
    let cleanup = () => {};
    getConfig()
      .then((config) => {
        applyAppConfig(config);
        cleanup = watchSystemTheme(config.theme as ThemeOption);
      })
      .catch(() => {});
    return () => cleanup();
  }, []);

  useEffect(() => {
    const unlisten = listen<AppConfig>("config-changed", (event) => {
      applyAppConfig(event.payload);
      watchSystemTheme(event.payload.theme as ThemeOption);
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
    </ContextMenuProvider>
  );
}

export default App;