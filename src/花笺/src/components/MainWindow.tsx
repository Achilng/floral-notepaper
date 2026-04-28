import { useCallback, useEffect, useMemo, useState } from "react";
import { MarkdownPreview } from "../features/markdown/MarkdownPreview";
import {
  createNote,
  deleteNote,
  getErrorMessage,
  getNote,
  listNotes,
  updateNote,
} from "../features/notes/api";
import type { Note, NoteMetadata } from "../features/notes/types";
import {
  countNoteChars,
  filterNotes,
  formatShortDate,
  formatTime,
  getDisplayTitle,
  metadataFromNote,
} from "../features/notes/noteUtils";
import { openNotepadWindow, openTileWindow } from "../features/windows/api";

type ViewMode = "edit" | "preview" | "split";
type SaveState = "idle" | "dirty" | "saving" | "saved" | "error";

const saveStateLabel: Record<SaveState, string> = {
  idle: "未选择",
  dirty: "未保存",
  saving: "保存中",
  saved: "已保存",
  error: "保存失败",
};

export function MainWindow() {
  const [notes, setNotes] = useState<NoteMetadata[]>([]);
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [searchQuery, setSearchQuery] = useState("");
  const [viewMode, setViewMode] = useState<ViewMode>("split");
  const [sidebarCollapsed, setSidebarCollapsed] = useState(false);
  const [content, setContent] = useState("");
  const [title, setTitle] = useState("");
  const [saveState, setSaveState] = useState<SaveState>("idle");
  const [hoveredId, setHoveredId] = useState<string | null>(null);
  const [isLoading, setIsLoading] = useState(true);
  const [errorMessage, setErrorMessage] = useState<string | null>(null);

  const selectedNote = useMemo(
    () => notes.find((note) => note.id === selectedId) ?? null,
    [notes, selectedId],
  );

  const filteredNotes = useMemo(
    () => filterNotes(notes, searchQuery),
    [notes, searchQuery],
  );

  const applyNote = useCallback((note: Note) => {
    setSelectedId(note.id);
    setTitle(note.title);
    setContent(note.content);
    setSaveState("saved");
    setErrorMessage(null);
  }, []);

  const replaceNoteMetadata = useCallback((note: Note) => {
    const metadata = metadataFromNote(note);
    setNotes((current) => {
      const exists = current.some((item) => item.id === metadata.id);
      const next = exists
        ? current.map((item) => (item.id === metadata.id ? metadata : item))
        : [metadata, ...current];
      return [...next].sort((left, right) =>
        right.updatedAt.localeCompare(left.updatedAt),
      );
    });
  }, []);

  const loadNote = useCallback(
    async (id: string) => {
      setErrorMessage(null);
      const note = await getNote(id);
      applyNote(note);
      replaceNoteMetadata(note);
    },
    [applyNote, replaceNoteMetadata],
  );

  const refreshNotes = useCallback(async () => {
    const loadedNotes = await listNotes();
    setNotes(loadedNotes);
    return loadedNotes;
  }, []);

  useEffect(() => {
    let cancelled = false;

    async function bootstrap() {
      setIsLoading(true);
      try {
        const loadedNotes = await listNotes();
        if (cancelled) return;
        setNotes(loadedNotes);
        if (loadedNotes[0]) {
          const note = await getNote(loadedNotes[0].id);
          if (!cancelled) applyNote(note);
        } else {
          setSelectedId(null);
          setTitle("");
          setContent("");
          setSaveState("idle");
        }
      } catch (error) {
        if (!cancelled) setErrorMessage(getErrorMessage(error));
      } finally {
        if (!cancelled) setIsLoading(false);
      }
    }

    void bootstrap();
    return () => {
      cancelled = true;
    };
  }, [applyNote]);

  const saveCurrentNote = useCallback(async () => {
    if (!selectedId) return null;

    setSaveState("saving");
    try {
      const note = await updateNote(selectedId, { title, content });
      replaceNoteMetadata(note);
      setSaveState("saved");
      setErrorMessage(null);
      return note;
    } catch (error) {
      setSaveState("error");
      setErrorMessage(getErrorMessage(error));
      return null;
    }
  }, [content, replaceNoteMetadata, selectedId, title]);

  useEffect(() => {
    if (!selectedId || saveState !== "dirty") return undefined;

    const timer = window.setTimeout(() => {
      void saveCurrentNote();
    }, 900);

    return () => window.clearTimeout(timer);
  }, [saveCurrentNote, saveState, selectedId]);

  const handleNewNote = async () => {
    setErrorMessage(null);
    try {
      const note = await createNote({ title: "", content: "" });
      replaceNoteMetadata(note);
      applyNote(note);
    } catch (error) {
      setErrorMessage(getErrorMessage(error));
    }
  };

  const handleSelectNote = async (id: string) => {
    if (id === selectedId) return;
    if (saveState === "dirty") {
      await saveCurrentNote();
    }

    setIsLoading(true);
    try {
      await loadNote(id);
    } catch (error) {
      setErrorMessage(getErrorMessage(error));
    } finally {
      setIsLoading(false);
    }
  };

  const handleDeleteNote = async () => {
    if (!selectedId) return;

    setErrorMessage(null);
    try {
      await deleteNote(selectedId);
      const remaining = await refreshNotes();
      if (remaining[0]) {
        await loadNote(remaining[0].id);
      } else {
        setSelectedId(null);
        setTitle("");
        setContent("");
        setSaveState("idle");
      }
    } catch (error) {
      setErrorMessage(getErrorMessage(error));
    }
  };

  const markDirty = () => {
    if (selectedId) setSaveState("dirty");
  };

  const handleOpenNotepad = async () => {
    setErrorMessage(null);
    try {
      await openNotepadWindow();
    } catch (error) {
      setErrorMessage(getErrorMessage(error));
    }
  };

  const handlePinEntry = async () => {
    if (!selectedId) return;
    if (saveState === "dirty") {
      await saveCurrentNote();
    }

    setErrorMessage(null);
    try {
      await openTileWindow(selectedId);
    } catch (error) {
      setErrorMessage(getErrorMessage(error));
    }
  };

  return (
    <div className="w-full max-w-6xl mx-auto animate-scale-in">
      <div
        className="noise-bg rounded-2xl bg-cloud border border-paper-deep/40 overflow-hidden flex flex-col"
        style={{
          height: "680px",
          boxShadow:
            "0 2px 8px rgba(26,26,24,0.04), 0 20px 60px rgba(26,26,24,0.08), 0 0 0 0.5px rgba(26,26,24,0.03)",
        }}
      >
        <div className="flex items-center justify-between px-4 h-11 bg-paper/60 border-b border-paper-deep/30 shrink-0 select-none">
          <div className="flex items-center gap-3 min-w-0">
            <div className="flex items-center gap-[7px] shrink-0">
              <div className="w-[11px] h-[11px] rounded-full bg-[#ff5f57] border border-[#e14640]/40" />
              <div className="w-[11px] h-[11px] rounded-full bg-[#febc2e] border border-[#d8a118]/40" />
              <div className="w-[11px] h-[11px] rounded-full bg-[#28c840] border border-[#1aab2e]/40" />
            </div>
            <div className="h-4 w-px bg-paper-deep/40" />
            <span className="text-[13px] font-display font-medium text-ink-soft tracking-wide">
              花笺
            </span>
            <span className="text-[11px] text-ink-ghost font-body">—</span>
            <span className="text-[11px] text-ink-faint font-body truncate max-w-[240px]">
              {title || selectedNote?.preview || "无标题笔记"}
            </span>
          </div>
          <div className="flex items-center gap-2">
            {errorMessage && (
              <span className="max-w-[260px] truncate text-[11px] text-red-400">
                {errorMessage}
              </span>
            )}
            <button
              onClick={() => void handleOpenNotepad()}
              className="w-7 h-7 flex items-center justify-center rounded-lg text-ink-ghost hover:text-bamboo hover:bg-bamboo-mist/50 transition-all"
              title="快捷便签"
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
                <path d="M4 4h16v14H7l-3 3V4z" />
                <path d="M8 9h8M8 13h5" />
              </svg>
            </button>
            <button
              className="w-7 h-7 flex items-center justify-center rounded-lg text-ink-ghost hover:text-ink-faint hover:bg-paper-warm transition-all"
              title="设置"
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
                <circle cx="12" cy="12" r="3" />
                <path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 0 1 0 2.83 2 2 0 0 1-2.83 0l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-2 2 2 2 0 0 1-2-2v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 0 1-2.83 0 2 2 0 0 1 0-2.83l.06-.06A1.65 1.65 0 0 0 4.68 15a1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1-2-2 2 2 0 0 1 2-2h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 0 1 0-2.83 2 2 0 0 1 2.83 0l.06.06A1.65 1.65 0 0 0 9 4.68a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 2-2 2 2 0 0 1 2 2v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 0 1 2.83 0 2 2 0 0 1 0 2.83l-.06.06A1.65 1.65 0 0 0 19.4 9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 2 2 2 2 0 0 1-2 2h-.09a1.65 1.65 0 0 0-1.51 1z" />
              </svg>
            </button>
          </div>
        </div>

        <div className="flex flex-1 min-h-0">
          <div
            className={`border-r border-paper-deep/30 bg-paper/40 flex flex-col shrink-0 transition-all duration-300 ${
              sidebarCollapsed ? "w-0 overflow-hidden" : "w-[280px]"
            }`}
          >
            <div className="px-3 pt-3 pb-2 shrink-0">
              <div className="flex items-center gap-2 px-2.5 h-8 rounded-lg bg-paper-warm/80 border border-paper-deep/40 focus-within:border-bamboo/30 focus-within:bg-cloud transition-all">
                <svg
                  width="13"
                  height="13"
                  viewBox="0 0 24 24"
                  fill="none"
                  stroke="currentColor"
                  strokeWidth="2.5"
                  strokeLinecap="round"
                  className="text-ink-ghost shrink-0"
                >
                  <circle cx="11" cy="11" r="8" />
                  <path d="m21 21-4.35-4.35" />
                </svg>
                <input
                  type="text"
                  value={searchQuery}
                  onChange={(event) => setSearchQuery(event.target.value)}
                  placeholder="搜索笔记…"
                  className="flex-1 text-[12px] font-body text-ink placeholder:text-ink-ghost/60 bg-transparent"
                />
                {searchQuery && (
                  <button
                    onClick={() => setSearchQuery("")}
                    className="text-ink-ghost hover:text-ink-faint transition-colors cursor-pointer"
                    title="清空搜索"
                  >
                    <svg
                      width="10"
                      height="10"
                      viewBox="0 0 24 24"
                      fill="none"
                      stroke="currentColor"
                      strokeWidth="3"
                      strokeLinecap="round"
                    >
                      <path d="M18 6L6 18M6 6l12 12" />
                    </svg>
                  </button>
                )}
              </div>
            </div>

            <div className="px-3 pb-2 shrink-0">
              <button
                onClick={handleNewNote}
                className="w-full flex items-center gap-2 px-2.5 py-1.5 rounded-lg text-[12px] font-body text-bamboo hover:bg-bamboo-mist/60 transition-all cursor-pointer group"
              >
                <svg
                  width="13"
                  height="13"
                  viewBox="0 0 24 24"
                  fill="none"
                  stroke="currentColor"
                  strokeWidth="2.5"
                  strokeLinecap="round"
                  className="group-hover:rotate-90 transition-transform duration-200"
                >
                  <path d="M12 5v14M5 12h14" />
                </svg>
                <span>新建笔记</span>
              </button>
            </div>

            <div className="px-5 pb-1.5 shrink-0">
              <span className="text-[10px] text-ink-ghost font-mono tracking-wider uppercase">
                {filteredNotes.length} 篇笔记
              </span>
            </div>

            <div className="flex-1 overflow-y-auto px-2 pb-2">
              <div className="space-y-0.5">
                {filteredNotes.map((note) => {
                  const isSelected = note.id === selectedId;
                  const isHovered = note.id === hoveredId;

                  return (
                    <button
                      key={note.id}
                      onClick={() => void handleSelectNote(note.id)}
                      onMouseEnter={() => setHoveredId(note.id)}
                      onMouseLeave={() => setHoveredId(null)}
                      className={`w-full text-left rounded-xl px-3 py-2.5 transition-all duration-200 cursor-pointer group relative ${
                        isSelected
                          ? "bg-bamboo-mist/70"
                          : isHovered
                            ? "bg-paper-warm/70"
                            : "bg-transparent"
                      }`}
                    >
                      {isSelected && (
                        <div className="absolute left-0 top-1/2 -translate-y-1/2 w-[3px] h-5 rounded-r-full bg-bamboo/60" />
                      )}

                      <div className="flex items-baseline justify-between mb-0.5">
                        <span
                          className={`text-[13px] font-display font-medium truncate pr-2 transition-colors ${
                            isSelected ? "text-bamboo" : "text-ink-soft"
                          }`}
                        >
                          {getDisplayTitle(note)}
                        </span>
                        <span className="text-[10px] text-ink-ghost font-mono tabular-nums shrink-0">
                          {formatShortDate(note.updatedAt)}
                        </span>
                      </div>

                      <p className="text-[11px] text-ink-ghost leading-relaxed line-clamp-2 group-hover:text-ink-faint transition-colors">
                        {note.preview || "空白笔记"}
                      </p>

                      <div className="flex items-center gap-2 mt-1">
                        <span className="text-[10px] text-ink-ghost/60 font-mono tabular-nums">
                          {formatTime(note.updatedAt)}
                        </span>
                        <span className="text-[10px] text-ink-ghost/40">·</span>
                        <span className="text-[10px] text-ink-ghost/60 font-mono tabular-nums">
                          {note.wordCount} 字
                        </span>
                      </div>
                    </button>
                  );
                })}

                {!isLoading && filteredNotes.length === 0 && (
                  <div className="px-3 py-8 text-center text-[12px] text-ink-ghost leading-relaxed">
                    {searchQuery ? "没有匹配的笔记" : "还没有笔记"}
                  </div>
                )}
              </div>
            </div>
          </div>

          <div className="flex-1 flex flex-col min-w-0">
            <div className="flex items-center justify-between px-4 h-10 border-b border-paper-deep/20 shrink-0 bg-paper/20">
              <div className="flex items-center gap-1">
                <button
                  onClick={() => setSidebarCollapsed(!sidebarCollapsed)}
                  className="w-7 h-7 flex items-center justify-center rounded-lg text-ink-ghost hover:text-ink-faint hover:bg-paper-warm transition-all cursor-pointer"
                  title={sidebarCollapsed ? "展开侧栏" : "收起侧栏"}
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
                    <rect x="3" y="3" width="18" height="18" rx="2" ry="2" />
                    <line x1="9" y1="3" x2="9" y2="21" />
                  </svg>
                </button>

                <div className="h-4 w-px bg-paper-deep/30 mx-1" />

                <button
                  onClick={() => void handlePinEntry()}
                  disabled={!selectedId}
                  className="w-7 h-7 flex items-center justify-center rounded-lg text-ink-ghost hover:text-bamboo hover:bg-bamboo-mist/50 transition-all cursor-pointer disabled:opacity-30 disabled:cursor-not-allowed"
                  title="钉为磁贴"
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
                    <path d="M12 17v5" />
                    <path d="M9 10.76a2 2 0 0 1-1.11 1.79l-1.78.9A2 2 0 0 0 5 15.24V16a1 1 0 0 0 1 1h12a1 1 0 0 0 1-1v-.76a2 2 0 0 0-1.11-1.79l-1.78-.9A2 2 0 0 1 15 10.76V7a1 1 0 0 1 1-1 1 1 0 0 0 1-1V4a1 1 0 0 0-1-1H8a1 1 0 0 0-1 1v1a1 1 0 0 0 1 1 1 1 0 0 1 1 1z" />
                  </svg>
                </button>

                <button
                  onClick={() => void saveCurrentNote()}
                  disabled={!selectedId || saveState === "saving"}
                  className="px-2.5 h-7 flex items-center justify-center rounded-lg text-[11px] text-ink-ghost hover:text-ink-faint hover:bg-paper-warm transition-all cursor-pointer disabled:opacity-30 disabled:cursor-not-allowed"
                  title="保存"
                >
                  保存
                </button>

                <button
                  onClick={() => void handleDeleteNote()}
                  disabled={!selectedId}
                  className="w-7 h-7 flex items-center justify-center rounded-lg text-ink-ghost hover:text-red-400 hover:bg-red-50 transition-all cursor-pointer disabled:opacity-30 disabled:cursor-not-allowed"
                  title="删除笔记"
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
                    <polyline points="3,6 5,6 21,6" />
                    <path d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6m3 0V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2" />
                  </svg>
                </button>
              </div>

              <div className="flex items-center bg-paper-warm/60 rounded-lg p-[2px] border border-paper-deep/30">
                {(["edit", "split", "preview"] as const).map((mode) => (
                  <button
                    key={mode}
                    onClick={() => setViewMode(mode)}
                    className={`px-3 py-1 rounded-md text-[11px] transition-all duration-200 cursor-pointer ${
                      viewMode === mode
                        ? "bg-cloud text-bamboo font-medium shadow-[0_1px_2px_rgba(0,0,0,0.04)]"
                        : "text-ink-ghost hover:text-ink-faint"
                    }`}
                  >
                    {mode === "edit" ? "编辑" : mode === "split" ? "分栏" : "预览"}
                  </button>
                ))}
              </div>
            </div>

            <div className="px-6 pt-4 pb-2 shrink-0 border-b border-paper-deep/15">
              <input
                type="text"
                value={title}
                onChange={(event) => {
                  setTitle(event.target.value);
                  markDirty();
                }}
                placeholder="无标题笔记"
                disabled={!selectedId}
                className="w-full text-[20px] font-display font-bold text-ink placeholder:text-ink-ghost/50 tracking-wide disabled:opacity-60"
              />
              <div className="flex items-center gap-3 mt-1.5">
                <span className="text-[10px] text-ink-ghost font-mono tabular-nums">
                  {selectedNote ? `${formatShortDate(selectedNote.updatedAt)} ${formatTime(selectedNote.updatedAt)}` : "--"}
                </span>
                <span className="text-[10px] text-ink-ghost/40">·</span>
                <span className="text-[10px] text-ink-ghost font-mono tabular-nums">
                  {countNoteChars(content)} 字
                </span>
                <span className="text-[10px] text-ink-ghost/40">·</span>
                <span
                  className={`text-[10px] font-mono tabular-nums ${
                    saveState === "error"
                      ? "text-red-400"
                      : saveState === "dirty"
                        ? "text-amber-500/70"
                        : "text-bamboo/60"
                  }`}
                >
                  {saveStateLabel[saveState]}
                </span>
              </div>
            </div>

            <div className="flex-1 flex min-h-0">
              {!selectedId && !isLoading ? (
                <div className="flex-1 flex items-center justify-center text-[13px] text-ink-ghost">
                  选择或新建一篇笔记
                </div>
              ) : (
                <>
                  {(viewMode === "edit" || viewMode === "split") && (
                    <div
                      className={`flex flex-col min-h-0 ${
                        viewMode === "split"
                          ? "w-1/2 border-r border-paper-deep/20"
                          : "w-full"
                      }`}
                    >
                      <div className="flex items-center gap-0.5 px-4 pt-2 pb-1 shrink-0">
                        {[
                          { label: "B", title: "粗体", style: "font-bold" },
                          { label: "I", title: "斜体", style: "italic" },
                          { label: "H", title: "标题", style: "font-bold" },
                          { label: "—", title: "分割线", style: "" },
                          { label: "•", title: "无序列表", style: "" },
                          { label: "1.", title: "有序列表", style: "font-mono text-[9px]" },
                          { label: "<>", title: "代码", style: "font-mono text-[9px]" },
                          { label: "❝", title: "引用", style: "" },
                        ].map((button) => (
                          <button
                            key={button.label}
                            title={button.title}
                            className={`w-6 h-6 flex items-center justify-center rounded text-[11px] text-ink-ghost hover:text-ink-faint hover:bg-paper-warm transition-all cursor-pointer ${button.style}`}
                          >
                            {button.label}
                          </button>
                        ))}
                      </div>

                      <div className="flex-1 overflow-y-auto px-5 pb-4">
                        <textarea
                          value={content}
                          onChange={(event) => {
                            setContent(event.target.value);
                            markDirty();
                          }}
                          className="w-full h-full text-[13.5px] leading-[1.9] text-ink-soft font-mono placeholder:text-ink-ghost/40"
                          placeholder="开始写作……"
                          spellCheck={false}
                          disabled={!selectedId}
                        />
                      </div>
                    </div>
                  )}

                  {(viewMode === "preview" || viewMode === "split") && (
                    <div
                      className={`flex flex-col min-h-0 ${
                        viewMode === "split" ? "w-1/2" : "w-full"
                      }`}
                    >
                      {viewMode === "split" && (
                        <div className="px-4 pt-2.5 pb-1 shrink-0">
                          <span className="text-[10px] text-ink-ghost/60 font-mono tracking-widest uppercase">
                            Preview
                          </span>
                        </div>
                      )}
                      <div
                        className={`flex-1 overflow-y-auto px-6 pb-6 ${
                          viewMode === "preview" ? "pt-3" : "pt-1"
                        }`}
                      >
                        <MarkdownPreview content={content} />
                      </div>
                    </div>
                  )}
                </>
              )}
            </div>

            <div className="flex items-center justify-between px-4 h-7 border-t border-paper-deep/20 bg-paper/30 shrink-0">
              <div className="flex items-center gap-3">
                <span className="text-[10px] text-ink-ghost font-mono tabular-nums">
                  Ln {content.split("\n").length}, Col 0
                </span>
                <span className="text-[10px] text-ink-ghost/40">|</span>
                <span className="text-[10px] text-ink-ghost font-mono">
                  Markdown
                </span>
              </div>
              <div className="flex items-center gap-3">
                <span className="text-[10px] text-ink-ghost font-mono">
                  UTF-8
                </span>
                <span className="text-[10px] text-ink-ghost/40">|</span>
                <span className="text-[10px] text-ink-ghost font-mono tabular-nums">
                  {(new TextEncoder().encode(content).length / 1024).toFixed(1)} KB
                </span>
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}
