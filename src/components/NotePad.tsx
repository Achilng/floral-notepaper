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
import { useTranslation } from "../features/i18n/LanguageContext";

type OpenMode = "new" | "open";

interface ToolbarButtonProps {
  onClick: () => void;
  label: string;
  tooltip?: string;
  tagMode?: boolean;
  children: React.ReactNode;
}

function ToolbarButton({ onClick, label, tooltip, tagMode, children }: ToolbarButtonProps) {
  return (
    <button
      type="button"
      onClick={onClick}
      title={`${label}${tooltip ? ` (${tooltip})` : ""}`}
      className={`w-6 h-6 flex items-center justify-center rounded text-[12px] transition-all duration-150 cursor-pointer font-medium ${
        tagMode
          ? "text-bamboo/70 hover:text-bamboo hover:bg-bamboo-mist/50"
          : "text-ink-faint hover:text-ink-soft hover:bg-paper-warm"
      }`}
    >
      {children}
    </button>
  );
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
    direction: "NorthWest",
    size: "w-8 h-8",
    className: "top-0 left-0 cursor-nwse-resize",
  },
  {
    direction: "NorthEast",
    size: "w-5 h-5",
    className: "top-0 right-0 cursor-nesw-resize",
  },
  {
    direction: "SouthWest",
    size: "w-8 h-8",
    className: "bottom-0 left-0 cursor-nesw-resize",
  },
  {
    direction: "SouthEast",
    size: "w-5 h-5",
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
  const { t } = useTranslation();
  const [surfaceMode, setSurfaceMode] =
    useState<NoteSurfaceMode>(initialSurfaceMode);
  const [mode, setMode] = useState<OpenMode>("new");
  const [notes, setNotes] = useState<NoteMetadata[]>([]);
  const [editingNoteId, setEditingNoteId] = useState<string | null>(null);
  const [title, setTitle] = useState("");
  const [content, setContent] = useState("");
  const [hoveredNote, setHoveredNote] = useState<string | null>(null);
  const [status, setStatus] = useState("empty");
  const statusLabels: Record<string, string> = {
    empty: t("notepad.status.empty"),
    opened: t("notepad.status.opened"),
    saved: t("notepad.status.saved"),
    unsaved: t("notepad.status.unsaved"),
    saveFailed: t("notepad.status.saveFailed"),
    copied: t("notepad.status.copied"),
  };
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
    setMode("new");
    setStatus("opened");
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
      setMode("new");
      setStatus("empty");
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
    setStatus("saved");
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
      setStatus("saveFailed");
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
      applyNote(note);
      await switchSurfaceMode("pad");
    } catch (error) {
      setErrorMessage(getErrorMessage(error));
    }
  };

  const handlePin = async () => {
    setErrorMessage(null);
    try {
      if (shouldSaveBeforeSwitchingToTile(noteSurfaceAutoSave)) {
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
        throw new Error(t("notepad.clipboardNotSupported"));
      }
      await clipboard.writeText(content);
      setStatus("copied");
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
    if (!noteSurfaceAutoSave || mode !== "new" || status !== "unsaved") {
      return undefined;
    }
    if (!hasDraftContent()) return undefined;

    const timer = window.setTimeout(() => {
      void handleSave();
    }, 900);

    return () => window.clearTimeout(timer);
  }, [handleSave, hasDraftContent, mode, noteSurfaceAutoSave, status]);

  const handleDrag = (event: MouseEvent<HTMLElement>) => {
    const target = event.target as HTMLElement;
    if (target.closest("button,input,textarea")) return;
    void startCurrentWindowDrag().catch(() => undefined);
  };

  const resetDraft = () => {
    setEditingNoteId(null);
    setTitle("");
    setContent("");
    setMode("new");
    setStatus("empty");
    setErrorMessage(null);
  };

  const insertFormat = useCallback(
    (prefix: string, suffix: string = prefix) => {
      const textarea = contentRef.current;
      if (!textarea) return;

      const start = textarea.selectionStart;
      const end = textarea.selectionEnd;
      const selectedText = content.slice(start, end);

      const before = content.slice(0, start);
      const after = content.slice(end);
      const insertion = `${prefix}${selectedText}${suffix}`;

      setContent(before + insertion + after);
      setStatus("unsaved");

      requestAnimationFrame(() => {
        textarea.focus();
        if (selectedText.length > 0) {
          textarea.selectionStart = start + prefix.length;
          textarea.selectionEnd = start + prefix.length + selectedText.length;
        } else {
          textarea.selectionStart = start + prefix.length;
          textarea.selectionEnd = start + prefix.length;
        }
      });
    },
    [content],
  );

  const insertInlineAtCursor = useCallback(
    (openTag: string, closeTag?: string) => {
      const textarea = contentRef.current;
      if (!textarea) return;

      const start = textarea.selectionStart;
      const end = textarea.selectionEnd;
      const selectedText = content.slice(start, end);

      const before = content.slice(0, start);
      const after = content.slice(end);

      if (closeTag && selectedText.length > 0) {
        setContent(before + openTag + selectedText + closeTag + after);
        setStatus("unsaved");
        requestAnimationFrame(() => {
          textarea.focus();
          textarea.selectionStart = start + openTag.length;
          textarea.selectionEnd = start + openTag.length + selectedText.length;
        });
      } else {
        const text = closeTag ? openTag + closeTag : openTag;
        setContent(before + text + after);
        setStatus("unsaved");
        requestAnimationFrame(() => {
          textarea.focus();
          const cursorPos = closeTag ? start + openTag.length : start + text.length;
          textarea.selectionStart = cursorPos;
          textarea.selectionEnd = cursorPos;
        });
      }
    },
    [content],
  );

  const isTile = surfaceMode === "tile";
  const tileNoteId = editingNoteId ?? initialNoteId ?? "";
  const tileTitle = title.trim();
  const enterClass = hasEnteredOnce.current ? "" : "animate-window-enter";
  const surfaceWrapperClassName = `w-full h-screen flex flex-col bg-transparent p-0 ${isExiting ? "animate-window-exit" : enterClass}`;
  const padSurfaceClassName =
    "relative noise-bg w-full h-full min-h-0 bg-cloud overflow-hidden flex flex-col flex-1 border border-paper-deep/40 rounded-xl shadow-[0_1px_10px_rgba(26,26,24,0.06)] transition-all duration-200 ease-out";

  return (
    <div className={surfaceWrapperClassName}>
      {isTile ? (
        <Tile
          title={tileTitle || undefined}
          content={errorMessage || content}
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
              className="flex items-center justify-between px-4 pt-3 pb-0 cursor-default"
              onMouseDown={handleDrag}
            >
              <div className="flex items-center gap-0.5">
                <button
                  onClick={resetDraft}
                  className={`relative px-3.5 py-1.5 text-[13px] rounded-t-lg transition-all duration-200 cursor-pointer ${
                    mode === "new"
                      ? "text-bamboo font-medium"
                      : "text-ink-ghost hover:text-ink-faint"
                  }`}
                >
                  {editingNoteId ? t("notepad.edit") : t("notepad.new")}
                  {mode === "new" && (
                    <div className="absolute bottom-0 left-3 right-3 h-[2px] bg-bamboo rounded-full" />
                  )}
                </button>
                <button
                  onClick={() => setMode("open")}
                  className={`relative px-3.5 py-1.5 text-[13px] rounded-t-lg transition-all duration-200 cursor-pointer ${
                    mode === "open"
                      ? "text-bamboo font-medium"
                      : "text-ink-ghost hover:text-ink-faint"
                  }`}
                >
                  {t("notepad.open")}
                  {mode === "open" && (
                    <div className="absolute bottom-0 left-3 right-3 h-[2px] bg-bamboo rounded-full" />
                  )}
                </button>
              </div>

              <div className="flex items-center gap-1.5">
                <button
                  onClick={() => void handlePin()}
                  className="group w-7 h-7 flex items-center justify-center rounded-lg transition-all duration-200 cursor-pointer text-ink-ghost hover:text-ink-faint hover:bg-paper-warm"
                  title={t("notepad.convertToTile")}
                >
                  <svg
                    width="14"
                    height="14"
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
                  className="group w-7 h-7 flex items-center justify-center rounded-lg text-ink-ghost hover:bg-danger-bg hover:text-red-400 transition-all duration-200 cursor-pointer"
                  title={t("notepad.close")}
                >
                  <svg
                    width="13"
                    height="13"
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

            <div className="mx-4 mt-1 h-px bg-paper-deep/50" />

            {mode === "new" ? (
              <div
                data-pad-editor-body="true"
                className="px-4 pt-3 pb-2 flex flex-col flex-1 min-h-0"
              >
                <input
                  type="text"
                  value={title}
                  onChange={(event) => {
                    setTitle(event.target.value);
                    setStatus("unsaved");
                  }}
                  placeholder={t("notepad.titlePlaceholder")}
                  className="w-full font-display font-medium text-ink placeholder:text-ink-ghost/60 mb-2 tracking-wide shrink-0"
                  style={{ fontSize: `${surfaceFontSize}px` }}
                />

                <div className="flex items-center gap-0.5 mb-2 shrink-0 flex-wrap">
                  <ToolbarButton
                    onClick={() => insertFormat("**", "**")}
                    label={t("toolbar.bold")}
                    tooltip="**"
                  >
                    B
                  </ToolbarButton>
                  <ToolbarButton
                    onClick={() => insertFormat("*", "*")}
                    label={t("toolbar.italic")}
                    tooltip="*"
                  >
                    <em className="font-normal not-italic">I</em>
                  </ToolbarButton>
                  <ToolbarButton
                    onClick={() => insertFormat("==", "==")}
                    label={t("toolbar.highlight")}
                    tooltip="=="
                  >
                    <span className="bg-bamboo-mist/80 rounded px-[3px] text-[11px]">H</span>
                  </ToolbarButton>
                  <ToolbarButton
                    onClick={() => insertFormat("~~", "~~")}
                    label={t("toolbar.strikethrough")}
                    tooltip="~~"
                  >
                    <span className="line-through">S</span>
                  </ToolbarButton>
                  <span className="w-px h-4 bg-paper-deep/40 mx-0.5" />
                  <ToolbarButton
                    onClick={() => insertFormat("`", "`")}
                    label={t("toolbar.inlineCode")}
                    tooltip="`"
                  >
                    <span className="font-mono text-[11px]">&lt;&gt;</span>
                  </ToolbarButton>
                  <ToolbarButton
                    onClick={() => insertInlineAtCursor("<sup>", "</sup>")}
                    label={t("toolbar.superscript")}
                    tagMode
                  >
                    <sup className="text-[10px] font-mono">sup</sup>
                  </ToolbarButton>
                  <ToolbarButton
                    onClick={() => insertInlineAtCursor("<sub>", "</sub>")}
                    label={t("toolbar.subscript")}
                    tagMode
                  >
                    <sub className="text-[10px] font-mono">sub</sub>
                  </ToolbarButton>
                  <span className="w-px h-4 bg-paper-deep/40 mx-0.5" />
                  <ToolbarButton
                    onClick={() => insertFormat("> ")}
                    label={t("toolbar.quote")}
                    tooltip="> "
                  >
                    <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5" strokeLinecap="round" strokeLinejoin="round">
                      <path d="M3 21c3 0 7-1 7-8V5c0-1.25-.756-2.017-2-2H4c-1.25 0-2 .75-2 1.972V11c0 1.25.75 2 2 2 1 0 1 0 1 1v1c0 1-1 2-2 2s-1 .008-1 1.031V21z"/>
                      <path d="M15 21c3 0 7-1 7-8V5c0-1.25-.757-2.017-2-2h-4c-1.25 0-2 .75-2 1.972V11c0 1.25.75 2 2 2h.75c0 2.25.25 4-2.75 4v3z"/>
                    </svg>
                  </ToolbarButton>
                  <ToolbarButton
                    onClick={() => {
                      const nextFootnoteId = (content.match(/\[\^(\d+)\]/g) ?? []).length + 1;
                      insertFormat(`[^${nextFootnoteId}]`);
                    }}
                    label={t("toolbar.footnote")}
                    tooltip="[^n]"
                  >
                    <span className="text-[10px] align-super">[^]</span>
                  </ToolbarButton>
                  <ToolbarButton
                    onClick={() => insertFormat("---\n")}
                    label={t("toolbar.hr")}
                    tooltip="---"
                  >
                    —
                  </ToolbarButton>
                </div>

                <textarea
                  ref={contentRef}
                  value={content}
                  onChange={(event) => {
                    setContent(event.target.value);
                    setStatus("unsaved");
                  }}
                  placeholder={t("notepad.contentPlaceholder")}
                  className="w-full flex-1 min-h-0 pb-2 leading-relaxed text-ink-soft font-body placeholder:text-ink-ghost/50"
                  style={{ fontSize: `${surfaceFontSize}px` }}
                />

                <div className="flex items-center justify-between mt-auto pt-2 border-t border-paper-deep/30 shrink-0">
                  <span className="text-[11px] text-ink-ghost font-mono tabular-nums truncate max-w-[170px]">
                    {errorMessage ?? `${t("notepad.charCount", { count: countNoteChars(content) })} · ${statusLabels[status] ?? status}`}
                  </span>
                  <div className="flex items-center gap-2">
                    <button
                      onClick={resetDraft}
                      className="px-4 py-1.5 text-[12px] text-ink-faint hover:text-ink-soft rounded-lg hover:bg-paper-warm transition-all duration-200 cursor-pointer"
                    >
                      {t("notepad.clear")}
                    </button>
                    <button
                      onClick={() => void handleSave()}
                      className="px-4 py-1.5 text-[12px] text-cloud bg-bamboo hover:bg-bamboo-light rounded-lg transition-all duration-200 font-medium cursor-pointer"
                    >
                      {t("notepad.save")}
                    </button>
                  </div>
                </div>
              </div>
            ) : (
              <div className="p-2 flex-1 min-h-0 overflow-y-auto">
                <div className="space-y-0.5">
                  {notes.map((note) => (
                    <button
                      key={note.id}
                      onClick={() => void handleOpenNote(note.id)}
                      onMouseEnter={() => setHoveredNote(note.id)}
                      onMouseLeave={() => setHoveredNote(null)}
                      className="w-full text-left px-3.5 py-3 rounded-xl transition-all duration-200 cursor-pointer group hover:bg-paper-warm/70"
                    >
                      <div className="flex items-baseline justify-between mb-0.5">
                        <span className="text-[13px] font-display font-medium text-ink-soft group-hover:text-ink transition-colors truncate pr-3">
                          {getDisplayTitle(note)}
                        </span>
                        <span className="text-[11px] text-ink-ghost font-mono tabular-nums">
                          {formatShortDate(note.updatedAt)}
                        </span>
                      </div>
                      <p className="text-[12px] text-ink-ghost leading-relaxed line-clamp-1 group-hover:text-ink-faint transition-colors">
                        {note.preview || t("notepad.blankNote")}
                      </p>
                      {hoveredNote === note.id && (
                        <div className="mt-1.5 h-px bg-bamboo/10 transition-all duration-300" />
                      )}
                    </button>
                  ))}
                  {notes.length === 0 && (
                    <div className="px-4 py-8 text-center text-[12px] text-ink-ghost">
                      {t("notepad.noNotesToOpen")}
                    </div>
                  )}
                </div>
              </div>
            )}
          </>
          <SurfaceResizeHandles />
        </div>
      )}
    </div>
  );
}
