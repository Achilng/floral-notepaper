import { useState } from "react";
import "./App.css";
import { MainWindow } from "./components/MainWindow";
import { NotePad } from "./components/NotePad";
import { TileShowcase } from "./components/TileShowcase";

type View = "main" | "notepad" | "tile";

function getInitialView(): View {
  const params = new URLSearchParams(window.location.search);
  const v = params.get("view");
  if (v === "tile") return "tile";
  if (v === "notepad") return "notepad";
  if (v === "main") return "main";
  return "main";
}

function App() {
  const [activeView, setActiveView] = useState<View>(getInitialView);

  const isMainView = activeView === "main";

  return (
    <div className="min-h-screen bg-paper font-body text-ink relative overflow-hidden">
      {/* Subtle dot grid background */}
      <div
        className="fixed inset-0 pointer-events-none opacity-[0.035]"
        style={{
          backgroundImage:
            "radial-gradient(circle, var(--color-ink-faint) 0.8px, transparent 0.8px)",
          backgroundSize: "32px 32px",
        }}
      />

      {/* Floating decorative blurs */}
      <div className="fixed top-16 right-24 w-40 h-40 rounded-full bg-bamboo-glow/25 blur-3xl animate-float" />
      <div
        className="fixed bottom-24 left-16 w-28 h-28 rounded-full bg-paper-deep/40 blur-2xl animate-float"
        style={{ animationDelay: "1.8s" }}
      />

      {/* Header */}
      <header className="relative z-10 flex items-center justify-between px-10 pt-6 pb-4">
        <div className="flex items-baseline gap-3">
          <h1 className="font-display text-[22px] font-bold tracking-tight text-ink">
            花笺
          </h1>
          <div className="h-3.5 w-px bg-ink-ghost/50" />
          <span className="text-[11px] text-ink-faint tracking-[0.15em] uppercase font-body">
            Design Preview
          </span>
        </div>

        {/* View switcher pill */}
        <div className="flex items-center bg-paper-warm/80 rounded-full p-[3px] border border-paper-deep/50 backdrop-blur-sm">
          <button
            onClick={() => setActiveView("main")}
            className={`px-5 py-1.5 rounded-full text-[13px] transition-all duration-300 cursor-pointer font-body ${
              activeView === "main"
                ? "bg-cloud text-bamboo font-medium shadow-[0_1px_3px_rgba(0,0,0,0.06)]"
                : "text-ink-faint hover:text-ink-soft"
            }`}
          >
            主窗口
          </button>
          <button
            onClick={() => setActiveView("notepad")}
            className={`px-5 py-1.5 rounded-full text-[13px] transition-all duration-300 cursor-pointer font-body ${
              activeView === "notepad"
                ? "bg-cloud text-bamboo font-medium shadow-[0_1px_3px_rgba(0,0,0,0.06)]"
                : "text-ink-faint hover:text-ink-soft"
            }`}
          >
            便签小窗
          </button>
          <button
            onClick={() => setActiveView("tile")}
            className={`px-5 py-1.5 rounded-full text-[13px] transition-all duration-300 cursor-pointer font-body ${
              activeView === "tile"
                ? "bg-cloud text-bamboo font-medium shadow-[0_1px_3px_rgba(0,0,0,0.06)]"
                : "text-ink-faint hover:text-ink-soft"
            }`}
          >
            磁贴
          </button>
        </div>
      </header>

      {/* Content stage */}
      <main
        className={`relative z-10 flex items-start justify-center px-6 pb-6 ${
          isMainView ? "min-h-[calc(100vh-72px)]" : "min-h-[calc(100vh-100px)] px-10 pb-10"
        }`}
      >
        <div
          key={activeView}
          className="animate-scale-in w-full flex items-start justify-center"
        >
          {activeView === "main" ? (
            <MainWindow />
          ) : activeView === "notepad" ? (
            <NotePad />
          ) : (
            <TileShowcase />
          )}
        </div>
      </main>
    </div>
  );
}

export default App;
