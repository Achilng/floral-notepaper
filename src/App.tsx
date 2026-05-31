import { getCurrentWindow } from "@tauri-apps/api/window";
import { invoke } from "@tauri-apps/api/core";
import { message } from "@tauri-apps/plugin-dialog";
import { useEffect } from "react";
import "./App.css";
import { AboutPanel } from "./components/AboutPanel";
import { ContextMenuProvider } from "./components/ContextMenu";
import { MainWindow } from "./components/MainWindow";
import { NotePad } from "./components/NotePad";
import { TileShowcase } from "./components/TileShowcase";
import { tabToIndentListener } from "indent-textarea";
import { getConfig } from "./features/settings/api";
import { applyTheme, watchSystemTheme } from "./features/settings/theme";
import type { AppConfig, ThemeOption } from "./features/settings/types";
import { getInitialRoute } from "./features/windows/windowRoutes";
import { syncLanguage } from "./locales";
import { listen } from "@tauri-apps/api/event";

function App() {
  const route = getInitialRoute();
  const activeView = route.view;

  const triggerInstallErrorDialog = () => {
    message("更新安装程序执行失败，请稍后重试", {
      title: "安装更新失败",
      kind: "error",
    }).catch((err) => console.error("dialog error:", err));
  };

  useEffect(() => {
    let cleanup = () => {};
    getConfig()
      .then((config) => {
        const theme = (config.theme || "system") as ThemeOption;
        applyTheme(theme);
        cleanup = watchSystemTheme(theme);
        document.documentElement.style.setProperty(
          "--tab-indent-size",
          String(config.tabIndentSize ?? 2),
        );
        void syncLanguage(config.locale);
      })
      .catch(() => {});
    return () => cleanup();
  }, []);

  useEffect(() => {
    let themeCleanup = () => {};
    const unlisten = listen<AppConfig>("config-changed", (event) => {
      const theme = (event.payload.theme || "system") as ThemeOption;
      applyTheme(theme);
      themeCleanup();
      themeCleanup = watchSystemTheme(theme);
      document.documentElement.style.setProperty(
        "--tab-indent-size",
        String(event.payload.tabIndentSize ?? 2),
      );
      void syncLanguage(event.payload.locale);
    });
    return () => {
      themeCleanup();
      void unlisten.then((fn) => fn());
    };
  }, []);

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

  useEffect(() => {
    if (!import.meta.env.DEV) return;
    (window as Record<string, unknown>).__debug = {
      showInstallError: triggerInstallErrorDialog,
      openAboutWindow: () => void invoke("open_debug_about_window"),
    };
    return () => {
      delete (window as Record<string, unknown>).__debug;
    };
  }, []);

  return (
    <ContextMenuProvider>
      <div className="app-window-shell h-screen font-body text-ink overflow-hidden">
        {activeView === "main" ? (
          <MainWindow />
        ) : activeView === "about" ? (
          <AboutPanel onClose={() => getCurrentWindow().close()} />
        ) : activeView === "notepad" ? (
          <NotePad initialNoteId={route.noteId} />
        ) : (
          <TileShowcase noteId={route.noteId} />
        )}
        {import.meta.env.DEV && (
          <div className="fixed bottom-2 right-2 z-50 flex gap-1 opacity-30 hover:opacity-100 transition-opacity">
            <button
              type="button"
              onClick={() => void invoke("open_debug_about_window")}
              className="h-6 px-2 rounded bg-ink-soft/80 text-[10px] text-cloud cursor-pointer"
            >
              关于预览
            </button>
            <button
              type="button"
              onClick={triggerInstallErrorDialog}
              className="h-6 px-2 rounded bg-red-500/80 text-[10px] text-white cursor-pointer"
            >
              安装失败弹窗
            </button>
          </div>
        )}
      </div>
    </ContextMenuProvider>
  );
}

export default App;
