import { useEffect, useState } from "react";
import "./App.css";
import { ContextMenuProvider } from "./components/ContextMenu";
import { MainWindow } from "./components/MainWindow";
import { NotePad } from "./components/NotePad";
import { TileShowcase } from "./components/TileShowcase";
import { LanguageContext, translate } from "./features/i18n/LanguageContext";
import type { Language } from "./features/i18n/types";
import { getConfig } from "./features/settings/api";
import { applyTheme, watchSystemTheme } from "./features/settings/theme";
import type { AppConfig, ThemeOption } from "./features/settings/types";
import { getInitialRoute } from "./features/windows/windowRoutes";
import { listen } from "@tauri-apps/api/event";

function App() {
  const route = getInitialRoute();
  const activeView = route.view;
  const [language, setLanguage] = useState<Language>("zh-CN");

  useEffect(() => {
    let cleanup = () => {};
    getConfig()
      .then((config) => {
        const theme = (config.theme || "system") as ThemeOption;
        applyTheme(theme);
        cleanup = watchSystemTheme(theme);
        if (config.language) {
          setLanguage(config.language);
        }
      })
      .catch(() => {});
    return () => cleanup();
  }, []);

  useEffect(() => {
    const unlisten = listen<AppConfig>("config-changed", (event) => {
      const theme = (event.payload.theme || "system") as ThemeOption;
      applyTheme(theme);
      watchSystemTheme(theme);
      if (event.payload.language) {
        setLanguage(event.payload.language);
      }
    });
    return () => {
      void unlisten.then((fn) => fn());
    };
  }, []);

  useEffect(() => {
    document.documentElement.lang = language;
  }, [language]);

  return (
    <LanguageContext.Provider
      value={{
        language,
        setLanguage,
        t: (key, params) => translate(language, key, params),
      }}
    >
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
    </LanguageContext.Provider>
  );
}

export default App;
