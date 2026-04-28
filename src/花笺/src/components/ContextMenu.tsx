import { useEffect, useRef, useState } from "react";
import { requestSurfaceAction } from "../features/windows/surfaceActions";
import { tileContextMenuItems } from "../features/windows/tileContextMenu";

interface MenuState {
  x: number;
  y: number;
  hasSelection: boolean;
  type: "edit" | "tile";
}

export function ContextMenuProvider({ children }: { children: React.ReactNode }) {
  const [menu, setMenu] = useState<MenuState | null>(null);
  const menuRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    function handleContextMenu(event: MouseEvent) {
      const target = event.target as HTMLElement;
      const isEditable =
        target.tagName === "TEXTAREA" ||
        target.tagName === "INPUT" ||
        target.isContentEditable;
      const tileTarget = target.closest<HTMLElement>('[data-context-menu="tile"]');

      if (!isEditable && !tileTarget) {
        event.preventDefault();
        return;
      }

      event.preventDefault();
      const selection = window.getSelection()?.toString() || "";

      let x = event.clientX;
      let y = event.clientY;
      const menuWidth = 160;
      const menuHeight = tileTarget ? 150 : 170;
      if (x + menuWidth > window.innerWidth) x = window.innerWidth - menuWidth - 4;
      if (y + menuHeight > window.innerHeight) y = window.innerHeight - menuHeight - 4;

      if (tileTarget) {
        setMenu({
          x,
          y,
          hasSelection: false,
          type: "tile",
        });
        return;
      }

      setMenu({ x, y, hasSelection: selection.length > 0, type: "edit" });
    }

    function handleClick() {
      setMenu(null);
    }

    function handleKeyDown(event: KeyboardEvent) {
      if (event.key === "Escape") setMenu(null);
    }

    document.addEventListener("contextmenu", handleContextMenu);
    document.addEventListener("mousedown", handleClick);
    document.addEventListener("keydown", handleKeyDown);
    return () => {
      document.removeEventListener("contextmenu", handleContextMenu);
      document.removeEventListener("mousedown", handleClick);
      document.removeEventListener("keydown", handleKeyDown);
    };
  }, []);

  const runCommand = (command: string) => {
    document.execCommand(command);
    setMenu(null);
  };

  const runSurfaceAction = (
    action: (typeof tileContextMenuItems)[number]["action"],
  ) => {
    requestSurfaceAction(action);
    setMenu(null);
  };

  const items = menu
    ? menu.type === "tile"
      ? tileContextMenuItems.map((item) => ({
          ...item,
          shortcut: "",
          action: () => runSurfaceAction(item.action),
          disabled: false,
        }))
      : [
          {
            label: "剪切",
            shortcut: "Ctrl+X",
            action: () => runCommand("cut"),
            disabled: !menu.hasSelection,
          },
          {
            label: "复制",
            shortcut: "Ctrl+C",
            action: () => runCommand("copy"),
            disabled: !menu.hasSelection,
          },
          {
            label: "粘贴",
            shortcut: "Ctrl+V",
            action: () => runCommand("paste"),
            disabled: false,
          },
          { separator: true as const },
          {
            label: "全选",
            shortcut: "Ctrl+A",
            action: () => runCommand("selectAll"),
            disabled: false,
          },
        ]
    : [];

  return (
    <>
      {children}
      {menu && (
        <div
          ref={menuRef}
          className="fixed z-[9999] min-w-[152px] py-1.5 bg-cloud/95 backdrop-blur-sm border border-paper-deep/50 rounded-lg overflow-hidden select-none"
          style={{
            left: menu.x,
            top: menu.y,
          }}
          onMouseDown={(event) => event.stopPropagation()}
        >
          {items.map((item, index) =>
            "separator" in item ? (
              <div key={index} className="mx-2 my-1 h-px bg-paper-deep/40" />
            ) : (
              <button
                key={item.label}
                onClick={() => void item.action()}
                disabled={item.disabled}
                className={`w-full flex items-center justify-between px-3 py-1.5 text-[12px] font-body transition-colors cursor-pointer disabled:text-ink-ghost/40 disabled:cursor-default disabled:hover:bg-transparent ${
                  "tone" in item && item.tone === "danger"
                    ? "text-red-400 hover:bg-red-50 hover:text-red-500"
                    : "text-ink-soft hover:bg-bamboo-mist/60 hover:text-bamboo"
                }`}
              >
                <span>{item.label}</span>
                {item.shortcut && (
                  <span className="text-[10px] text-ink-ghost/60 font-mono ml-6">
                    {item.shortcut}
                  </span>
                )}
              </button>
            ),
          )}
        </div>
      )}
    </>
  );
}
