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
  const activeView = getInitialView();

  const isMainView = activeView === "main";

  return (
    <div className="min-h-screen bg-paper font-body text-ink relative overflow-hidden">
      <div
        className="fixed inset-0 pointer-events-none opacity-[0.035]"
        style={{
          backgroundImage:
            "radial-gradient(circle, var(--color-ink-faint) 0.8px, transparent 0.8px)",
          backgroundSize: "32px 32px",
        }}
      />

      <main
        className={`relative z-10 flex items-start justify-center px-6 pb-6 ${
          isMainView ? "min-h-screen py-6" : "min-h-screen px-10 py-10"
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
