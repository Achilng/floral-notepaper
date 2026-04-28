import { useCallback, useEffect, useState } from "react";
import type { MouseEvent } from "react";
import { getErrorMessage, getNote, updateNote } from "../features/notes/api";
import type { Note } from "../features/notes/types";
import { countNoteChars, formatShortDate } from "../features/notes/noteUtils";
import {
  closeCurrentWindow,
  setCurrentWindowAlwaysOnTop,
  startCurrentWindowDrag,
  startCurrentWindowResize,
} from "../features/windows/controls";

interface TileShowcaseProps {
  noteId?: string;
}

export function TileShowcase({ noteId }: TileShowcaseProps) {
  const [note, setNote] = useState<Note | null>(null);
  const [title, setTitle] = useState("");
  const [content, setContent] = useState("");
  const [isDirty, setIsDirty] = useState(false);
  const [isAlwaysOnTop, setIsAlwaysOnTop] = useState(true);
  const [status, setStatus] = useState("加载中");
  const [errorMessage, setErrorMessage] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;

    async function loadTileNote() {
      if (!noteId) {
        setStatus("缺少笔记");
        return;
      }

      try {
        const loaded = await getNote(noteId);
        if (cancelled) return;
        setNote(loaded);
        setTitle(loaded.title);
        setContent(loaded.content);
        setStatus("已载入");
        setErrorMessage(null);
      } catch (error) {
        if (!cancelled) {
          setStatus("载入失败");
          setErrorMessage(getErrorMessage(error));
        }
      }
    }

    void loadTileNote();
    void setCurrentWindowAlwaysOnTop(true).catch(() => undefined);

    return () => {
      cancelled = true;
    };
  }, [noteId]);

  const saveTile = useCallback(async () => {
    if (!note) return;

    setStatus("保存中");
    try {
      const saved = await updateNote(note.id, { title, content });
      setNote(saved);
      setIsDirty(false);
      setStatus("已保存");
      setErrorMessage(null);
    } catch (error) {
      setStatus("保存失败");
      setErrorMessage(getErrorMessage(error));
    }
  }, [content, note, title]);

  useEffect(() => {
    if (!note || !isDirty) return undefined;

    const timer = window.setTimeout(() => {
      void saveTile();
    }, 900);

    return () => window.clearTimeout(timer);
  }, [isDirty, note, saveTile]);

  const markDirty = () => {
    setIsDirty(true);
    setStatus("未保存");
  };

  const handleDrag = (event: MouseEvent<HTMLDivElement>) => {
    if ((event.target as HTMLElement).closest("button, input, textarea")) return;
    void startCurrentWindowDrag().catch(() => undefined);
  };

  const toggleAlwaysOnTop = async () => {
    const next = !isAlwaysOnTop;
    setIsAlwaysOnTop(next);
    try {
      await setCurrentWindowAlwaysOnTop(next);
    } catch (error) {
      setErrorMessage(getErrorMessage(error));
    }
  };

  const handleClose = async () => {
    if (isDirty) {
      await saveTile();
    }

    try {
      await closeCurrentWindow();
    } catch (error) {
      setErrorMessage(getErrorMessage(error));
    }
  };

  return (
    <div className="w-full h-screen flex flex-col">
      <div
        className="noise-bg relative bg-bamboo-mist overflow-hidden flex flex-col flex-1"
        onMouseDown={handleDrag}
      >
        <div className="flex items-center gap-1.5 px-3 pt-2 pb-1 cursor-grab active:cursor-grabbing shrink-0">
          <svg width="10" height="14" viewBox="0 0 10 14" className="text-bamboo-light/40 shrink-0 mr-1">
            <circle cx="2.5" cy="2.5" r="1.2" fill="currentColor" />
            <circle cx="7.5" cy="2.5" r="1.2" fill="currentColor" />
            <circle cx="2.5" cy="7" r="1.2" fill="currentColor" />
            <circle cx="7.5" cy="7" r="1.2" fill="currentColor" />
            <circle cx="2.5" cy="11.5" r="1.2" fill="currentColor" />
            <circle cx="7.5" cy="11.5" r="1.2" fill="currentColor" />
          </svg>

          <input
            value={title}
            onChange={(event) => {
              setTitle(event.target.value);
              markDirty();
            }}
            placeholder="无标题磁贴"
            className="flex-1 min-w-0 text-[12px] font-display text-ink-faint/80 tracking-wide bg-transparent"
          />

          <button
            onClick={() => void toggleAlwaysOnTop()}
            className={`w-6 h-6 flex items-center justify-center rounded-md transition-all cursor-pointer ${
              isAlwaysOnTop
                ? "text-bamboo bg-bamboo-glow/60"
                : "text-ink-ghost hover:text-ink-faint hover:bg-paper-warm"
            }`}
            title={isAlwaysOnTop ? "取消置顶" : "置顶"}
          >
            <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
              <path d="M12 17v5" />
              <path d="M9 10.76a2 2 0 0 1-1.11 1.79l-1.78.9A2 2 0 0 0 5 15.24V16a1 1 0 0 0 1 1h12a1 1 0 0 0 1-1v-.76a2 2 0 0 0-1.11-1.79l-1.78-.9A2 2 0 0 1 15 10.76V7a1 1 0 0 1 1-1 1 1 0 0 0 1-1V4a1 1 0 0 0-1-1H8a1 1 0 0 0-1 1v1a1 1 0 0 0 1 1 1 1 0 0 1 1 1z" />
            </svg>
          </button>
          <button
            onClick={() => void handleClose()}
            className="w-6 h-6 flex items-center justify-center rounded-md text-ink-ghost hover:text-red-500 hover:bg-red-50 transition-all cursor-pointer"
            title="关闭"
          >
            <svg width="11" height="11" viewBox="0 0 12 12" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round">
              <path d="M2 2l8 8M10 2l-8 8" />
            </svg>
          </button>
        </div>

        <div className="px-3.5 pt-1 pb-3 flex-1 min-h-0 flex flex-col">
          <textarea
            value={content}
            onChange={(event) => {
              setContent(event.target.value);
              markDirty();
            }}
            placeholder="磁贴内容"
            className="w-full flex-1 min-h-0 text-[13px] leading-[1.7] text-ink-soft/80 whitespace-pre-wrap font-body bg-transparent placeholder:text-ink-ghost/50"
          />
        </div>

        <div className="flex items-center justify-between px-3.5 py-2 border-t border-bamboo-glow/40">
          <span className="text-[10px] text-ink-ghost font-mono truncate max-w-[200px]">
            {errorMessage ??
              `${note ? formatShortDate(note.updatedAt) : "--"} · ${countNoteChars(content)} 字 · ${status}`}
          </span>
          <button
            onClick={() => void saveTile()}
            disabled={!note}
            className="text-[11px] text-bamboo hover:text-bamboo-light disabled:opacity-40"
          >
            保存
          </button>
        </div>

        <button
          onMouseDown={(event) => {
            event.stopPropagation();
            void startCurrentWindowResize().catch(() => undefined);
          }}
          className="absolute bottom-0 right-0 w-6 h-6 cursor-se-resize text-ink-faint/50"
          title="调整大小"
        >
          <svg
            className="absolute bottom-1.5 right-1.5"
            width="9"
            height="9"
            viewBox="0 0 9 9"
            fill="currentColor"
          >
            <circle cx="7" cy="7" r="1" />
            <circle cx="7" cy="3.5" r="0.8" opacity="0.5" />
            <circle cx="3.5" cy="7" r="0.8" opacity="0.5" />
          </svg>
        </button>
      </div>
    </div>
  );
}
