import { useCallback, useEffect, useState } from "react";
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
import { openTileWindow } from "../features/windows/api";
import {
  closeCurrentWindow,
  startCurrentWindowDrag,
} from "../features/windows/controls";

type Mode = "new" | "open";

interface NotePadProps {
  initialNoteId?: string;
}

export function NotePad({ initialNoteId }: NotePadProps) {
  const [mode, setMode] = useState<Mode>(initialNoteId ? "new" : "new");
  const [notes, setNotes] = useState<NoteMetadata[]>([]);
  const [editingNoteId, setEditingNoteId] = useState<string | null>(null);
  const [title, setTitle] = useState("");
  const [content, setContent] = useState("");
  const [isPinned, setIsPinned] = useState(false);
  const [hoveredNote, setHoveredNote] = useState<string | null>(null);
  const [status, setStatus] = useState("空");
  const [errorMessage, setErrorMessage] = useState<string | null>(null);

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
    setStatus("已打开");
  }, []);

  useEffect(() => {
    let cancelled = false;

    async function bootstrap() {
      try {
        await refreshNotes();
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

  const saveNote = async () => {
    const request = { title, content };
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
  };

  const handleSave = async () => {
    setErrorMessage(null);
    try {
      await saveNote();
    } catch (error) {
      setStatus("保存失败");
      setErrorMessage(getErrorMessage(error));
    }
  };

  const handleOpenNote = async (noteId: string) => {
    setErrorMessage(null);
    try {
      const note = await getNote(noteId);
      applyNote(note);
    } catch (error) {
      setErrorMessage(getErrorMessage(error));
    }
  };

  const handlePin = async () => {
    setErrorMessage(null);
    try {
      const note = await saveNote();
      await openTileWindow(note.id);
      setIsPinned(true);
      setStatus("已钉住");
    } catch (error) {
      setErrorMessage(getErrorMessage(error));
    }
  };

  const handleClose = async () => {
    try {
      await closeCurrentWindow();
    } catch (error) {
      setErrorMessage(getErrorMessage(error));
    }
  };

  const handleDrag = (event: MouseEvent<HTMLDivElement>) => {
    if ((event.target as HTMLElement).closest("button")) return;
    void startCurrentWindowDrag().catch(() => undefined);
  };

  const resetDraft = () => {
    setEditingNoteId(null);
    setTitle("");
    setContent("");
    setMode("new");
    setStatus("空");
    setErrorMessage(null);
  };

  return (
    <div className="flex flex-col items-center gap-5 pt-4 w-full max-w-md mx-auto">
      <div
        className="noise-bg w-full rounded-2xl bg-cloud border border-paper-deep/40 overflow-hidden"
        style={{
          boxShadow:
            "0 2px 8px rgba(26,26,24,0.04), 0 12px 40px rgba(26,26,24,0.07), 0 0 0 0.5px rgba(26,26,24,0.03)",
        }}
      >
        <div
          className="flex items-center justify-between px-5 pt-4 pb-0 cursor-grab active:cursor-grabbing"
          onMouseDown={handleDrag}
        >
          <div className="flex items-center gap-0.5">
            <button
              onClick={resetDraft}
              className={`relative px-3.5 py-1.5 text-[13px] rounded-t-lg transition-all duration-250 cursor-pointer ${
                mode === "new"
                  ? "text-bamboo font-medium"
                  : "text-ink-ghost hover:text-ink-faint"
              }`}
            >
              {editingNoteId ? "编辑" : "新建"}
              {mode === "new" && (
                <div className="absolute bottom-0 left-3 right-3 h-[2px] bg-bamboo rounded-full" />
              )}
            </button>
            <button
              onClick={() => setMode("open")}
              className={`relative px-3.5 py-1.5 text-[13px] rounded-t-lg transition-all duration-250 cursor-pointer ${
                mode === "open"
                  ? "text-bamboo font-medium"
                  : "text-ink-ghost hover:text-ink-faint"
              }`}
            >
              打开
              {mode === "open" && (
                <div className="absolute bottom-0 left-3 right-3 h-[2px] bg-bamboo rounded-full" />
              )}
            </button>
          </div>

          <div className="flex items-center gap-1.5">
            <button
              onClick={() => void handlePin()}
              className={`group w-7 h-7 flex items-center justify-center rounded-lg transition-all duration-200 cursor-pointer ${
                isPinned
                  ? "bg-bamboo-mist text-bamboo"
                  : "text-ink-ghost hover:text-ink-faint hover:bg-paper-warm"
              }`}
              title="钉为磁贴"
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
                className={`transition-transform duration-200 ${isPinned ? "rotate-[-45deg]" : ""}`}
              >
                <path d="M12 17v5" />
                <path d="M9 10.76a2 2 0 0 1-1.11 1.79l-1.78.9A2 2 0 0 0 5 15.24V16a1 1 0 0 0 1 1h12a1 1 0 0 0 1-1v-.76a2 2 0 0 0-1.11-1.79l-1.78-.9A2 2 0 0 1 15 10.76V7a1 1 0 0 1 1-1 1 1 0 0 0 1-1V4a1 1 0 0 0-1-1H8a1 1 0 0 0-1 1v1a1 1 0 0 0 1 1 1 1 0 0 1 1 1z" />
              </svg>
            </button>

            <button
              onClick={() => void handleClose()}
              className="group w-7 h-7 flex items-center justify-center rounded-lg text-ink-ghost hover:bg-red-50 hover:text-red-400 transition-all duration-200 cursor-pointer"
              title="关闭"
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

        <div className="mx-5 mt-1.5 h-px bg-paper-deep/50" />

        {mode === "new" ? (
          <div className="px-5 py-4">
            <input
              type="text"
              value={title}
              onChange={(event) => {
                setTitle(event.target.value);
                setStatus("未保存");
              }}
              placeholder="标题（可选）"
              className="w-full text-[15px] font-display font-medium text-ink placeholder:text-ink-ghost/60 mb-3 tracking-wide"
            />

            <textarea
              value={content}
              onChange={(event) => {
                setContent(event.target.value);
                setStatus("未保存");
              }}
              placeholder="写点什么……"
              className="w-full h-48 text-[14px] leading-relaxed text-ink-soft font-body placeholder:text-ink-ghost/50"
            />

            <div className="flex items-center justify-between mt-2 pt-3 border-t border-paper-deep/30">
              <span className="text-[11px] text-ink-ghost font-mono tabular-nums truncate max-w-[170px]">
                {errorMessage ?? `${countNoteChars(content)} 字 · ${status}`}
              </span>
              <div className="flex items-center gap-2">
                <button
                  onClick={resetDraft}
                  className="px-4 py-1.5 text-[12px] text-ink-faint hover:text-ink-soft rounded-lg hover:bg-paper-warm transition-all duration-200 cursor-pointer"
                >
                  清空
                </button>
                <button
                  onClick={() => void handleSave()}
                  className="px-4 py-1.5 text-[12px] text-cloud bg-bamboo hover:bg-bamboo-light rounded-lg transition-all duration-200 font-medium cursor-pointer shadow-[0_1px_3px_rgba(45,90,61,0.2)]"
                >
                  保存
                </button>
              </div>
            </div>
          </div>
        ) : (
          <div className="p-2 max-h-[310px] overflow-y-auto">
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
                    {note.preview || "空白笔记"}
                  </p>
                  {hoveredNote === note.id && (
                    <div className="mt-1.5 h-px bg-bamboo/10 transition-all duration-300" />
                  )}
                </button>
              ))}
              {notes.length === 0 && (
                <div className="px-4 py-8 text-center text-[12px] text-ink-ghost">
                  还没有可打开的笔记
                </div>
              )}
            </div>
          </div>
        )}
      </div>
    </div>
  );
}
