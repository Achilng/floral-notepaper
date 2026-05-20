import { useCallback, useEffect, useRef, useState } from "react";
import type { MouseEvent } from "react";
import {
  createNote,
  getErrorMessage,
  getNote,
  listNotes,
  updateNote,
} from "../features/notes/api";
import type { Note, NoteMetadata } from "../features/notes/types";
import {
  countNoteChars,
  formatShortDate,
  getDisplayTitle,
  metadataFromNote,
} from "../features/notes/noteUtils";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import {
  animateCurrentWindowBounds,
  getCurrentWindowBounds,
  recycleCurrentNotepad,
  setCurrentWindowAlwaysOnTop,
  showCurrentWindow,
  startCurrentWindowDrag,
  startCurrentWindowResize,
} from "../features/windows/controls";
import { openNoteInEditor } from "../features/windows/api";
import type { ResizeDirection } from "../features/windows/controls";
import { getConfig } from "../features/settings/api";
import {
  DEFAULT_TILE_COLOR,
  normalizeTileColor,
  resolveTileColor,
} from "../features/settings/tileColor";
import type { TileColorMode } from "../features/settings/types";
import { shouldSaveBeforeSwitchingToTile } from "../features/windows/noteSurfaceSavePolicy";
import {
  NOTE_SURFACE_ACTION_EVENT,
  surfaceActionFromEvent,
} from "../features/windows/surfaceActions";
import {
  NOTE_SURFACE_MODE_EVENT,
  getSurfaceTargetBounds,
  surfaceModeFromEvent,
} from "../features/windows/surfaceMode";
import type { NoteSurfaceMode } from "../features/windows/surfaceMode";
import { Tile } from "./Tile";
import { MilkdownPreview } from "../features/editor/MilkdownPreview";

type ActiveTab = "new" | "open" | string;

interface OpenedTab {
  noteId: string;
  title: string;
  content: string;
}

interface NotePadProps {
  initialNoteId?: string;
  initialSurfaceMode?: NoteSurfaceMode;
  initialAutoSave?: boolean;
  initialTileColor?: string;
}

const surfaceResizeHandles: Array<{
  direction: ResizeDirection;
  className: string;
  size: string;
}> = [
  {
    direction: "North",
    size: "w-full h-2",
    className: "top-0 left-0 cursor-ns-resize",
  },
  {
    direction: "South",
    size: "w-full h-2",
    className: "bottom-0 left-0 cursor-ns-resize",
  },
  {
    direction: "West",
    size: "h-full w-2",
    className: "top-0 left-0 cursor-ew-resize",
  },
  {
    direction: "East",
    size: "h-full w-2",
    className: "top-0 right-0 cursor-ew-resize",
  },
  {
    direction: "NorthWest",
    size: "w-3 h-3",
    className: "top-0 left-0 cursor-nwse-resize",
  },
  {
    direction: "NorthEast",
    size: "w-3 h-3",
    className: "top-0 right-0 cursor-nesw-resize",
  },
  {
    direction: "SouthWest",
    size: "w-3 h-3",
    className: "bottom-0 left-0 cursor-nesw-resize",
  },
  {
    direction: "SouthEast",
    size: "w-3 h-3",
    className: "bottom-0 right-0 cursor-nwse-resize",
  },
];

function SurfaceResizeHandles() {
  return (
    <>
      {surfaceResizeHandles.map((handle) => (
        <div
          key={handle.direction}
          aria-hidden="true"
          data-surface-resize-handle="true"
          data-resize-direction={handle.direction}
          onMouseDown={(event) => {
            event.stopPropagation();
            void startCurrentWindowResize(handle.direction).catch(
              () => undefined,
            );
          }}
          className={`absolute ${handle.size} opacity-0 ${handle.className}`}
        />
      ))}
    </>
  );
}

export function NotePad({
  initialNoteId,
  initialSurfaceMode = "pad",
  initialAutoSave = true,
  initialTileColor = DEFAULT_TILE_COLOR,
}: NotePadProps) {
  const [surfaceMode, setSurfaceMode] =
    useState<NoteSurfaceMode>(initialSurfaceMode);
  const [activeTab, setActiveTab] = useState<ActiveTab>("new");
  const [openedTabs, setOpenedTabs] = useState<OpenedTab[]>([]);
  const [notes, setNotes] = useState<NoteMetadata[]>([]);
  const [editingNoteId, setEditingNoteId] = useState<string | null>(null);
  const [title, setTitle] = useState("");
  const [content, setContent] = useState("");
  const [openSearch, setOpenSearch] = useState("");
  const [status, setStatus] = useState("空");
  const [errorMessage, setErrorMessage] = useState<string | null>(null);
  const [noteSurfaceAutoSave, setNoteSurfaceAutoSave] =
    useState(initialAutoSave);
  const [tileColorRaw, setTileColorRaw] = useState(
    normalizeTileColor(initialTileColor),
  );
  const [tileColorMode, setTileColorMode] = useState<TileColorMode>("system");
  const [surfaceFontSize, setSurfaceFontSize] = useState(14);
  const [tileColor, setTileColor] = useState(() =>
    resolveTileColor("system", normalizeTileColor(initialTileColor)),
  );
  const [isExiting, setIsExiting] = useState(false);
  const contentRef = useRef<HTMLTextAreaElement>(null);
  const isStandby = useRef(
    typeof window !== "undefined" &&
      new URLSearchParams(window.location.search).get("standby") === "1",
  );
  const hasEnteredOnce = useRef(false);

  const refreshNotes = useCallback(async () => {
    const loadedNotes = await listNotes();
    setNotes(loadedNotes);
    return loadedNotes;
  }, []);

  const applyNote = useCallback((note: Note) => {
    setEditingNoteId(note.id);
    setTitle(note.title);
    setContent(note.content);
    setActiveTab("new");
    setStatus("已打开");
  }, []);

  useEffect(() => {
    let cancelled = false;

    async function bootstrap() {
      try {
        const [loadedConfig] = await Promise.all([getConfig(), refreshNotes()]);
        if (!cancelled) {
          setNoteSurfaceAutoSave(loadedConfig.noteSurfaceAutoSave);
          setSurfaceFontSize(loadedConfig.surfaceFontSize ?? 14);
          setTileColorRaw(normalizeTileColor(loadedConfig.tileColor));
          setTileColorMode(loadedConfig.tileColorMode ?? "system");
          setTileColor(
            resolveTileColor(
              loadedConfig.tileColorMode ?? "system",
              loadedConfig.tileColor,
            ),
          );
        }
        if (initialNoteId) {
          const note = await getNote(initialNoteId);
          if (!cancelled) applyNote(note);
        }
      } catch (error) {
        if (!cancelled) setErrorMessage(getErrorMessage(error));
      }
    }

    void bootstrap();
    return () => {
      cancelled = true;
    };
  }, [applyNote, initialNoteId, refreshNotes]);

  useEffect(() => {
    const unlisten = listen("notes-changed", () => {
      void refreshNotes().catch(() => undefined);
    });
    return () => {
      void unlisten.then((fn) => fn());
    };
  }, [refreshNotes]);

  useEffect(() => {
    if (isStandby.current) return;
    let cancelled = false;
    requestAnimationFrame(() => {
      requestAnimationFrame(() => {
        if (!cancelled) {
          hasEnteredOnce.current = true;
          void showCurrentWindow()
            .then(() => contentRef.current?.focus())
            .catch(() => undefined);
        }
      });
    });
    return () => {
      cancelled = true;
    };
  }, []);

  useEffect(() => {
    const unlisten = listen<{
      tileColor?: string;
      tileColorMode?: TileColorMode;
      surfaceFontSize?: number;
    }>("config-changed", (event) => {
      const mode = event.payload.tileColorMode ?? tileColorMode;
      const raw = event.payload.tileColor ?? tileColorRaw;
      setTileColorMode(mode);
      setTileColorRaw(normalizeTileColor(raw));
      setTileColor(resolveTileColor(mode, raw));
      if (event.payload.surfaceFontSize != null) setSurfaceFontSize(event.payload.surfaceFontSize);
    });
    return () => {
      void unlisten.then((fn) => fn());
    };
  }, [tileColorMode, tileColorRaw]);

  useEffect(() => {
    if (tileColorMode !== "system") return;
    const observer = new MutationObserver(() => {
      setTileColor(resolveTileColor("system", tileColorRaw));
    });
    observer.observe(document.documentElement, {
      attributes: true,
      attributeFilter: ["data-theme"],
    });
    return () => observer.disconnect();
  }, [tileColorMode, tileColorRaw]);

  useEffect(() => {
    let myLabel = "";
    try {
      myLabel = getCurrentWindow().label;
    } catch {
      // not in Tauri environment (tests)
    }

    const unlisten = listen<string>("notepad:activate", (event) => {
      if (event.payload !== myLabel) return;

      isStandby.current = false;
      hasEnteredOnce.current = true;
      setEditingNoteId(null);
      setTitle("");
      setContent("");
      setActiveTab("new");
      setOpenedTabs([]);
      setStatus("空");
      setErrorMessage(null);
      setIsExiting(false);
      setSurfaceMode("pad");
      void refreshNotes().catch(() => undefined);
      void showCurrentWindow()
        .then(() => contentRef.current?.focus())
        .catch(() => undefined);
    });
    return () => {
      void unlisten.then((fn) => fn());
    };
  }, [refreshNotes]);

  const saveNote = useCallback(async () => {
    const existingCategory = notes.find((n) => n.id === editingNoteId)?.category ?? "";
    const request = { title, content, category: existingCategory };
    const note = editingNoteId
      ? await updateNote(editingNoteId, request)
      : await createNote(request);

    setEditingNoteId(note.id);
    setNotes((current) => {
      const metadata = metadataFromNote(note);
      const exists = current.some((item) => item.id === note.id);
      const next = exists
        ? current.map((item) => (item.id === note.id ? metadata : item))
        : [metadata, ...current];
      return [...next].sort((left, right) =>
        right.updatedAt.localeCompare(left.updatedAt),
      );
    });
    setStatus("已保存");
    return note;
  }, [content, editingNoteId, title]);

  const hasDraftContent = useCallback(
    () => Boolean(editingNoteId || title.trim() || content.trim()),
    [content, editingNoteId, title],
  );

  const switchSurfaceMode = useCallback(async (nextMode: NoteSurfaceMode) => {
    setSurfaceMode(nextMode);

    try {
      if (nextMode === "tile") {
        await setCurrentWindowAlwaysOnTop(true);
      }

      const currentBounds = await getCurrentWindowBounds();
      await animateCurrentWindowBounds(
        getSurfaceTargetBounds(nextMode, currentBounds),
      );
    } catch (error) {
      setErrorMessage(getErrorMessage(error));
    }
  }, []);

  useEffect(() => {
    function handleSurfaceModeRequest(event: Event) {
      const nextMode = surfaceModeFromEvent(event);
      if (!nextMode) return;
      void switchSurfaceMode(nextMode);
    }

    window.addEventListener(NOTE_SURFACE_MODE_EVENT, handleSurfaceModeRequest);
    return () => {
      window.removeEventListener(
        NOTE_SURFACE_MODE_EVENT,
        handleSurfaceModeRequest,
      );
    };
  }, [switchSurfaceMode]);

  useEffect(() => {
    if (surfaceMode !== "tile") return;
    void setCurrentWindowAlwaysOnTop(true).catch(() => undefined);
  }, [surfaceMode]);

  const handleSave = useCallback(async () => {
    setErrorMessage(null);
    try {
      await saveNote();
    } catch (error) {
      setStatus("保存失败");
      setErrorMessage(getErrorMessage(error));
    }
  }, [saveNote]);

  useEffect(() => {
    function handleKeyDown(event: KeyboardEvent) {
      if ((event.ctrlKey || event.metaKey) && event.key === "s") {
        event.preventDefault();
        void handleSave();
      }
    }

    document.addEventListener("keydown", handleKeyDown);
    return () => document.removeEventListener("keydown", handleKeyDown);
  }, [handleSave]);

  const handleOpenNote = async (noteId: string) => {
    setErrorMessage(null);
    try {
      const note = await getNote(noteId);
      setOpenedTabs((tabs) => {
        const exists = tabs.some((t) => t.noteId === noteId);
        if (exists) return tabs.map((t) => t.noteId === noteId ? { ...t, title: note.title, content: note.content } : t);
        return [...tabs, { noteId: note.id, title: note.title, content: note.content }];
      });
      setActiveTab(noteId);
      await switchSurfaceMode("pad");
    } catch (error) {
      setErrorMessage(getErrorMessage(error));
    }
  };

  const handlePin = async () => {
    setErrorMessage(null);
    try {
      const currentTab = activeTab !== "new" && activeTab !== "open"
        ? openedTabs.find((t) => t.noteId === activeTab)
        : undefined;
      if (currentTab) {
        setEditingNoteId(currentTab.noteId);
        setTitle(currentTab.title);
        setContent(currentTab.content);
      } else if (shouldSaveBeforeSwitchingToTile(noteSurfaceAutoSave)) {
        await saveNote();
      }
      await switchSurfaceMode("tile");
    } catch (error) {
      setErrorMessage(getErrorMessage(error));
    }
  };

  const handleClose = useCallback(() => {
    setIsExiting(true);
    void recycleCurrentNotepad().catch((error) => {
      setIsExiting(false);
      setErrorMessage(getErrorMessage(error));
    });
  }, []);

  const copyTileContent = useCallback(async () => {
    setErrorMessage(null);
    try {
      const clipboard = navigator.clipboard;
      if (!clipboard?.writeText) {
        throw new Error("当前环境不支持复制");
      }
      await clipboard.writeText(content);
      setStatus("已复制");
    } catch (error) {
      setErrorMessage(getErrorMessage(error));
    }
  }, [content]);

  useEffect(() => {
    function handleSurfaceActionRequest(event: Event) {
      const action = surfaceActionFromEvent(event);
      if (!action) return;

      if (action === "copy") {
        void copyTileContent();
        return;
      }

      if (action === "save") {
        void handleSave();
        return;
      }

      if (action === "close") {
        void handleClose();
        return;
      }

      void switchSurfaceMode("pad");
    }

    window.addEventListener(
      NOTE_SURFACE_ACTION_EVENT,
      handleSurfaceActionRequest,
    );
    return () => {
      window.removeEventListener(
        NOTE_SURFACE_ACTION_EVENT,
        handleSurfaceActionRequest,
      );
    };
  }, [copyTileContent, handleClose, handleSave, switchSurfaceMode]);

  useEffect(() => {
    if (!noteSurfaceAutoSave || activeTab !== "new" || status !== "未保存") {
      return undefined;
    }
    if (!hasDraftContent()) return undefined;

    const timer = window.setTimeout(() => {
      void handleSave();
    }, 900);

    return () => window.clearTimeout(timer);
  }, [handleSave, hasDraftContent, activeTab, noteSurfaceAutoSave, status]);

  const handleDrag = (event: MouseEvent<HTMLElement>) => {
    const target = event.target as HTMLElement;
    if (target.closest("button,input,textarea")) return;
    void startCurrentWindowDrag().catch(() => undefined);
  };

  const resetDraft = () => {
    setEditingNoteId(null);
    setTitle("");
    setContent("");
    setActiveTab("new");
    setStatus("空");
    setErrorMessage(null);
  };

  const isTile = surfaceMode === "tile";
  const tileNoteId = editingNoteId ?? initialNoteId ?? "";
  const tileTitle = title.trim();
  const enterClass = hasEnteredOnce.current ? "" : "animate-window-enter";
  const surfaceWrapperClassName = `w-full h-screen flex flex-col bg-transparent p-0 ${isExiting ? "animate-window-exit" : enterClass}`;
  const padSurfaceClassName =
    "relative noise-bg w-full h-full min-h-0 bg-paper overflow-hidden flex flex-col flex-1 border border-paper-deep/40 rounded-xl shadow-[0_1px_10px_rgba(26,26,24,0.06)] transition-all duration-200 ease-out";

  return (
    <div className={surfaceWrapperClassName}>
      {isTile ? (
        <Tile
          title={tileTitle || undefined}
          content={errorMessage || content}
          richContent={!errorMessage && content ? <MilkdownPreview content={content} fontSize={surfaceFontSize} readonly /> : undefined}
          color={tileColor}
          fontSize={surfaceFontSize}
          width="100%"
          className="h-full cursor-default"
          data-surface-mode={surfaceMode}
          data-context-menu="tile"
          data-note-id={tileNoteId}
          onMouseDown={handleDrag}
        >
          <SurfaceResizeHandles />
        </Tile>
      ) : (
        <div className={padSurfaceClassName} data-surface-mode={surfaceMode}>
          <>
            <div
              className="flex items-center justify-between px-1.5 pt-1.5 pb-0 cursor-default gap-1"
              onMouseDown={handleDrag}
            >
              <div className="flex items-center gap-0 min-w-0 flex-1 overflow-x-auto scrollbar-none">
                <button
                  onClick={resetDraft}
                  className={`relative px-2.5 py-1 text-[12px] rounded-t-lg transition-all duration-200 cursor-pointer shrink-0 ${
                    activeTab === "new"
                      ? "text-bamboo font-medium"
                      : "text-ink-ghost hover:text-ink-faint"
                  }`}
                >
                  {editingNoteId ? "编辑" : "新建"}
                  {activeTab === "new" && (
                    <div className="absolute bottom-0 left-2.5 right-2.5 h-[2px] bg-bamboo rounded-full" />
                  )}
                </button>
                <button
                  onClick={() => { setActiveTab("open"); setOpenSearch(""); }}
                  className={`relative px-2.5 py-1 text-[12px] rounded-t-lg transition-all duration-200 cursor-pointer shrink-0 ${
                    activeTab === "open"
                      ? "text-bamboo font-medium"
                      : "text-ink-ghost hover:text-ink-faint"
                  }`}
                >
                  打开
                  {activeTab === "open" && (
                    <div className="absolute bottom-0 left-2.5 right-2.5 h-[2px] bg-bamboo rounded-full" />
                  )}
                </button>
                {openedTabs.map((tab) => (
                  <button
                    key={tab.noteId}
                    onClick={() => setActiveTab(tab.noteId)}
                    className={`relative flex items-center gap-1 px-2 py-1 text-[11px] rounded-t-lg transition-all duration-200 cursor-pointer min-w-0 ${
                      activeTab === tab.noteId
                        ? "text-bamboo font-medium"
                        : "text-ink-ghost hover:text-ink-faint"
                    }`}
                    style={{ maxWidth: `${Math.max(60, 160 / Math.max(openedTabs.length, 1))}px` }}
                  >
                    <span className="truncate">{tab.title || "无标题"}</span>
                    <span
                      onClick={(e) => {
                        e.stopPropagation();
                        setOpenedTabs((tabs) => tabs.filter((t) => t.noteId !== tab.noteId));
                        if (activeTab === tab.noteId) setActiveTab("open");
                      }}
                      className="shrink-0 w-3.5 h-3.5 flex items-center justify-center rounded hover:bg-paper-deep/40 text-ink-ghost hover:text-ink-faint transition-colors"
                    >
                      <svg width="8" height="8" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="3" strokeLinecap="round">
                        <path d="M18 6L6 18M6 6l12 12" />
                      </svg>
                    </span>
                    {activeTab === tab.noteId && (
                      <div className="absolute bottom-0 left-2 right-2 h-[2px] bg-bamboo rounded-full" />
                    )}
                  </button>
                ))}
              </div>

              <div className="flex items-center gap-1.5 shrink-0">
                <button
                  onClick={() => void handlePin()}
                  className="group w-6 h-6 flex items-center justify-center rounded-lg transition-all duration-200 cursor-pointer text-ink-ghost hover:text-ink-faint hover:bg-paper-warm"
                  title="转为磁贴"
                >
                  <svg
                    width="12"
                    height="12"
                    viewBox="0 0 24 24"
                    fill="none"
                    stroke="currentColor"
                    strokeWidth="2"
                    strokeLinecap="round"
                    strokeLinejoin="round"
                  >
                    <path d="M12 17v5" />
                    <path d="M9 10.76a2 2 0 0 1-1.11 1.79l-1.78.9A2 2 0 0 0 5 15.24V16a1 1 0 0 0 1 1h12a1 1 0 0 0 1-1v-.76a2 2 0 0 0-1.11-1.79l-1.78-.9A2 2 0 0 1 15 10.76V7a1 1 0 0 1 1-1 1 1 0 0 0 1-1V4a1 1 0 0 0-1-1H8a1 1 0 0 0-1 1v1a1 1 0 0 0 1 1 1 1 0 0 1 1 1z" />
                  </svg>
                </button>

                <button
                  onClick={() => void handleClose()}
                  className="group w-6 h-6 flex items-center justify-center rounded-lg text-ink-ghost hover:bg-danger-bg hover:text-red-400 transition-all duration-200 cursor-pointer"
                  title="关闭"
                >
                  <svg
                    width="11"
                    height="11"
                    viewBox="0 0 24 24"
                    fill="none"
                    stroke="currentColor"
                    strokeWidth="2.5"
                    strokeLinecap="round"
                  >
                    <path d="M18 6L6 18M6 6l12 12" />
                  </svg>
                </button>
              </div>
            </div>

            <div className="mx-3 mt-0.5 h-px bg-paper-deep/50" />

            {activeTab === "new" ? (
              <div
                data-pad-editor-body="true"
                className="px-3 pt-2 pb-1 flex flex-col flex-1 min-h-0"
              >
                <input
                  type="text"
                  value={title}
                  onChange={(event) => {
                    setTitle(event.target.value);
                    setStatus("未保存");
                  }}
                  placeholder="标题（可选）"
                  className="w-full font-display font-medium text-ink placeholder:text-ink-ghost/60 mb-1 tracking-wide shrink-0"
                  style={{ fontSize: `${surfaceFontSize}px` }}
                />

                <textarea
                  ref={contentRef}
                  value={content}
                  onChange={(event) => {
                    setContent(event.target.value);
                    setStatus("未保存");
                  }}
                  placeholder="写点什么……"
                  className="w-full flex-1 min-h-0 pb-2 leading-relaxed text-ink-soft font-body placeholder:text-ink-ghost/50"
                  style={{ fontSize: `${surfaceFontSize}px` }}
                />

                <div className="flex items-center justify-between mt-auto pt-1 border-t border-paper-deep/30 shrink-0">
                  <span className="text-[10px] text-ink-ghost font-mono tabular-nums truncate max-w-[170px]">
                    {errorMessage ?? `${countNoteChars(content)} 字 · ${status}`}
                  </span>
                  <div className="flex items-center gap-2">
                    <button
                      onClick={resetDraft}
                      className="px-3 py-1 text-[11px] text-ink-faint hover:text-ink-soft rounded-lg hover:bg-paper-warm transition-all duration-200 cursor-pointer"
                    >
                      清空
                    </button>
                    <button
                      onClick={() => void handleSave()}
                      className="px-3 py-1 text-[11px] text-cloud bg-bamboo hover:bg-bamboo-light rounded-lg transition-all duration-200 font-medium cursor-pointer"
                    >
                      保存
                    </button>
                  </div>
                </div>
              </div>
            ) : activeTab === "open" ? (
              <div className="flex-1 min-h-0 flex flex-col">
                <div className="px-2 pt-1.5 pb-1 shrink-0">
                  <div className="flex items-center gap-1.5 px-2 h-7 rounded-lg bg-paper-warm/80 border border-paper-deep/40 focus-within:border-bamboo/30 focus-within:bg-paper transition-all">
                    <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5" strokeLinecap="round" className="text-ink-ghost shrink-0">
                      <circle cx="11" cy="11" r="8" />
                      <path d="m21 21-4.35-4.35" />
                    </svg>
                    <input
                      type="text"
                      value={openSearch}
                      onChange={(e) => setOpenSearch(e.target.value)}
                      placeholder="搜索笔记…"
                      className="flex-1 text-[11px] font-body text-ink placeholder:text-ink-ghost/60 bg-transparent"
                    />
                    {openSearch && (
                      <button
                        onClick={() => setOpenSearch("")}
                        className="text-ink-ghost hover:text-ink-faint transition-colors cursor-pointer"
                      >
                        <svg width="10" height="10" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5" strokeLinecap="round">
                          <path d="M18 6L6 18M6 6l12 12" />
                        </svg>
                      </button>
                    )}
                  </div>
                </div>
                <div className="px-2 pb-2 flex-1 min-h-0 overflow-y-auto">
                <div className="space-y-0.5">
                  {notes.filter((n) => {
                    if (!openSearch) return true;
                    const q = openSearch.toLowerCase();
                    return getDisplayTitle(n).toLowerCase().includes(q) || (n.preview ?? "").toLowerCase().includes(q);
                  }).map((note) => (
                    <button
                      key={note.id}
                      onClick={() => void handleOpenNote(note.id)}
                      className="w-full text-left px-3 py-1.5 rounded-lg transition-all duration-200 cursor-pointer group hover:bg-paper-warm/70"
                    >
                      <div className="flex items-center justify-between mb-0.5">
                        <span className="text-[12px] font-display font-medium text-ink-soft group-hover:text-ink transition-colors truncate pr-2">
                          {getDisplayTitle(note)}
                        </span>
                        <div className="flex items-center gap-1.5 shrink-0">
                          <button
                            onClick={(e) => {
                              e.stopPropagation();
                              void openNoteInEditor(note.id);
                            }}
                            className="w-6 h-6 flex items-center justify-center rounded-md text-ink-ghost hover:text-bamboo hover:bg-bamboo-mist/50 transition-all duration-200 opacity-0 group-hover:opacity-100 cursor-pointer"
                            title="在编辑器中打开"
                          >
                            <svg
                              width="13"
                              height="13"
                              viewBox="0 0 24 24"
                              fill="none"
                              stroke="currentColor"
                              strokeWidth="2"
                              strokeLinecap="round"
                              strokeLinejoin="round"
                            >
                              <path d="M18 13v6a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V8a2 2 0 0 1 2-2h6" />
                              <polyline points="15 3 21 3 21 9" />
                              <line x1="10" y1="14" x2="21" y2="3" />
                            </svg>
                          </button>
                          <span className="text-[10px] text-ink-ghost font-mono tabular-nums">
                            {formatShortDate(note.updatedAt)}
                          </span>
                        </div>
                      </div>
                      <p className="text-[11px] text-ink-ghost leading-relaxed line-clamp-1 group-hover:text-ink-faint transition-colors">
                        {note.preview || "空白笔记"}
                      </p>
                    </button>
                  ))}
                  {notes.length === 0 && (
                    <div className="px-4 py-8 text-center text-[12px] text-ink-ghost">
                      还没有可打开的笔记
                    </div>
                  )}
                </div>
                </div>
              </div>
            ) : (
              (() => {
                const currentTab = openedTabs.find((t) => t.noteId === activeTab);
                if (!currentTab) return null;
                return (
                  <div className="px-3 pt-2 pb-1 flex flex-col flex-1 min-h-0">
                    <div className="flex-1 min-h-0 overflow-y-auto">
                      <MilkdownPreview content={currentTab.content} fontSize={surfaceFontSize} readonly />
                    </div>
                    <div className="flex items-center justify-between mt-auto pt-1 border-t border-paper-deep/30 shrink-0">
                      <span className="text-[10px] text-ink-ghost font-mono tabular-nums truncate">
                        {countNoteChars(currentTab.content)} 字
                      </span>
                      <button
                        onClick={() => {
                          setEditingNoteId(currentTab.noteId);
                          setTitle(currentTab.title);
                          setContent(currentTab.content);
                          setActiveTab("new");
                          setStatus("已打开");
                        }}
                        className="px-3 py-1 text-[11px] text-cloud bg-bamboo hover:bg-bamboo-light rounded-lg transition-all duration-200 font-medium cursor-pointer"
                      >
                        编辑
                      </button>
                    </div>
                  </div>
                );
              })()
            )}
          </>
          <SurfaceResizeHandles />
        </div>
      )}
    </div>
  );
}
