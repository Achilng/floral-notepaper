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
    <div className="w-full max-w-[360px] mx-auto pt-4">
      <div
        className="noise-bg relative rounded-xl bg-bamboo-mist border border-bamboo-glow/60 overflow-hidden shadow-[0_8px_30px_rgba(26,26,24,0.1)]"
        onMouseDown={handleDrag}
      >
        <div className="flex items-center gap-2 px-3 pt-2.5 pb-1 cursor-grab active:cursor-grabbing">
          <div className="flex gap-[3px] text-bamboo-light/50">
            {[...Array(6)].map((_, index) => (
              <div
                key={index}
                className="w-[3px] h-[3px] rounded-full bg-current opacity-60"
                style={{ marginTop: index % 2 === 0 ? 0 : 4 }}
              />
            ))}
          </div>

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
            className={`w-5 h-5 rounded-full transition-all ${
              isAlwaysOnTop ? "bg-bamboo/50" : "bg-ink-ghost/25"
            }`}
            title="置顶"
          />
          <button
            onClick={() => void handleClose()}
            className="w-5 h-5 rounded-full bg-red-300/50 hover:bg-red-400/80 transition-all"
            title="关闭"
          />
        </div>

        <div className="px-3.5 pt-1 pb-3">
          <textarea
            value={content}
            onChange={(event) => {
              setContent(event.target.value);
              markDirty();
            }}
            placeholder="磁贴内容"
            className="w-full min-h-[190px] text-[13px] leading-[1.7] text-ink-soft/80 whitespace-pre-wrap font-body bg-transparent placeholder:text-ink-ghost/50"
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
