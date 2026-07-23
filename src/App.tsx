import { useCallback, useEffect, useRef } from "react";
import "./App.css";
import { ContextMenuProvider } from "./components/ContextMenu";
import { MainWindow } from "./components/MainWindow";
import { NotePad } from "./components/NotePad";
import { TileShowcase } from "./components/TileShowcase";
import { ToastContainer } from "./components/Toast";
import { tabToIndentListener } from "indent-textarea";
import { getConfig } from "./features/settings/api";
import { applyTheme, watchSystemTheme } from "./features/settings/theme";
import type { AppConfig, ThemeOption } from "./features/settings/types";

const DEFAULT_INTERFACE_FONT = "HarmonyOS Sans SC";
const DEFAULT_INTERFACE_BACKGROUND = "#f6f3ec";

function applyInterfaceSettings(
  config: Pick<AppConfig, "interfaceFontFamily" | "backgroundColor">,
) {
  const fontFamily = config.interfaceFontFamily?.trim() || DEFAULT_INTERFACE_FONT;
  const backgroundColor = config.backgroundColor?.trim() || DEFAULT_INTERFACE_BACKGROUND;
  document.documentElement.style.setProperty("--font-body", fontFamily);
  document.documentElement.style.setProperty("--font-display", fontFamily);
  document.documentElement.style.setProperty("--interface-background-color", backgroundColor);
}
import { getInitialRoute } from "./features/windows/windowRoutes";
import { syncLanguage } from "./locales";
import { listen } from "@tauri-apps/api/event";

function App() {
  const route = getInitialRoute();
  const activeView = route.view;
  const themeCleanupRef = useRef<() => void>(() => {});
  const applyConfig = useCallback((config: AppConfig) => {
    const theme = (config.theme || "system") as ThemeOption;
    applyTheme(theme);
    themeCleanupRef.current();
    themeCleanupRef.current = watchSystemTheme(theme);
    document.documentElement.style.setProperty(
      "--tab-indent-size",
      String(config.tabIndentSize ?? 2),
    );
    applyInterfaceSettings(config);
    void syncLanguage(config.locale);
  }, []);

  useEffect(() => {
    let cancelled = false;
    getConfig()
      .then((config) => {
        if (!cancelled) applyConfig(config);
      })
      .catch(() => {});
    return () => {
      cancelled = true;
    };
  }, [applyConfig]);

  useEffect(() => {
    const unlisten = listen<AppConfig>("config-changed", (event) => {
      applyConfig(event.payload);
    });
    return () => {
      themeCleanupRef.current();
      themeCleanupRef.current = () => {};
      void unlisten.then((fn) => fn());
    };
  }, [applyConfig]);

  useEffect(() => {
    const handleTab = (event: KeyboardEvent) => {
      const target = event.target;
      if (!(target instanceof HTMLTextAreaElement)) return;
      if (target.dataset.tabIndent !== "true") return;
      tabToIndentListener(event);
    };
    window.addEventListener("keydown", handleTab, true);
    return () => window.removeEventListener("keydown", handleTab, true);
  }, []);

  useEffect(() => {
    const isWindows =
      navigator.userAgent.includes("Windows") || navigator.platform.toLowerCase().startsWith("win");
    if (!isWindows) return;

    const preventSystemMenu = (e: KeyboardEvent) => {
      if (e.altKey && e.code === "Space") {
        e.preventDefault();
      }
    };
    document.addEventListener("keydown", preventSystemMenu, true);
    return () => document.removeEventListener("keydown", preventSystemMenu, true);
  }, []);

  return (
    <ContextMenuProvider>
      <div className="app-window-shell h-screen font-body text-ink overflow-hidden">
        {activeView === "main" ? (
          <MainWindow />
        ) : activeView === "notepad" ? (
          <NotePad initialNoteId={route.noteId} />
        ) : (
          <TileShowcase noteId={route.noteId} />
        )}
        <ToastContainer />
      </div>
    </ContextMenuProvider>
  );
}

export default App;
