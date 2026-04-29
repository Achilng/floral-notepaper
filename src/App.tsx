import "./App.css";
import { ContextMenuProvider } from "./components/ContextMenu";
import { MainWindow } from "./components/MainWindow";
import { NotePad } from "./components/NotePad";
import { TileShowcase } from "./components/TileShowcase";
import { getInitialRoute } from "./features/windows/windowRoutes";

function App() {
  const route = getInitialRoute();
  const activeView = route.view;

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
