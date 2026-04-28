import "./App.css";
import { MainWindow } from "./components/MainWindow";
import { NotePad } from "./components/NotePad";
import { TileShowcase } from "./components/TileShowcase";
import { getInitialRoute } from "./features/windows/windowRoutes";

function App() {
  const route = getInitialRoute();
  const activeView = route.view;

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
            <NotePad initialNoteId={route.noteId} />
          ) : (
            <TileShowcase noteId={route.noteId} />
          )}
        </div>
      </main>
    </div>
  );
}

export default App;
