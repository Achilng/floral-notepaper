import { useState } from "react";

type Mode = "new" | "open";

const existingNotes = [
  { id: 1, title: "读书笔记：月亮与六便士", date: "04-26", preview: "满地都是六便士，他却抬头看见了月亮……" },
  { id: 2, title: "周末采购清单", date: "04-25", preview: "牛奶、面包、橄榄油、番茄" },
  { id: 3, title: "项目灵感", date: "04-24", preview: "试试用 WebGL 做一个粒子书法效果？" },
  { id: 4, title: "给自己的信", date: "04-22", preview: "不要急，慢慢来。所有的经历都会变成养分……" },
];

export function NotePad() {
  const [mode, setMode] = useState<Mode>("new");
  const [title, setTitle] = useState("");
  const [content, setContent] = useState("");
  const [isPinned, setIsPinned] = useState(false);
  const [hoveredNote, setHoveredNote] = useState<number | null>(null);

  return (
    <div className="flex flex-col items-center gap-8 pt-8 w-full max-w-md mx-auto">
      {/* Keyboard shortcut hint */}
      <div className="flex items-center gap-2 text-ink-faint">
        <kbd className="px-2 py-0.5 text-[11px] font-mono bg-paper-warm border border-paper-deep rounded-[4px] text-ink-faint tracking-wide">
          Ctrl
        </kbd>
        <span className="text-[10px]">+</span>
        <kbd className="px-2 py-0.5 text-[11px] font-mono bg-paper-warm border border-paper-deep rounded-[4px] text-ink-faint tracking-wide">
          Space
        </kbd>
        <span className="text-[12px] ml-1.5">呼出便签</span>
      </div>

      {/* NotePad window */}
      <div
        className="noise-bg w-full rounded-2xl bg-cloud border border-paper-deep/40 overflow-hidden"
        style={{
          boxShadow:
            "0 2px 8px rgba(26,26,24,0.04), 0 12px 40px rgba(26,26,24,0.07), 0 0 0 0.5px rgba(26,26,24,0.03)",
        }}
      >
        {/* Top bar with tabs */}
        <div className="flex items-center justify-between px-5 pt-4 pb-0">
          {/* Mode tabs */}
          <div className="flex items-center gap-0.5">
            <button
              onClick={() => setMode("new")}
              className={`relative px-3.5 py-1.5 text-[13px] rounded-t-lg transition-all duration-250 cursor-pointer ${
                mode === "new"
                  ? "text-bamboo font-medium"
                  : "text-ink-ghost hover:text-ink-faint"
              }`}
            >
              新建
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

          {/* Window controls */}
          <div className="flex items-center gap-1.5">
            {/* Pin button */}
            <button
              onClick={() => setIsPinned(!isPinned)}
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

            {/* Close button */}
            <button
              className="group w-7 h-7 flex items-center justify-center rounded-lg text-ink-ghost hover:bg-red-50 hover:text-red-400 transition-all duration-200 cursor-pointer"
              title="关闭"
            >
              <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5" strokeLinecap="round">
                <path d="M18 6L6 18M6 6l12 12" />
              </svg>
            </button>
          </div>
        </div>

        {/* Divider */}
        <div className="mx-5 mt-1.5 h-px bg-paper-deep/50" />

        {/* Content area */}
        {mode === "new" ? (
          <div className="px-5 py-4">
            {/* Title input */}
            <input
              type="text"
              value={title}
              onChange={(e) => setTitle(e.target.value)}
              placeholder="标题（可选）"
              className="w-full text-[15px] font-display font-medium text-ink placeholder:text-ink-ghost/60 mb-3 tracking-wide"
            />

            {/* Body textarea */}
            <textarea
              value={content}
              onChange={(e) => setContent(e.target.value)}
              placeholder="写点什么……"
              className="w-full h-48 text-[14px] leading-relaxed text-ink-soft font-body placeholder:text-ink-ghost/50"
            />

            {/* Bottom bar */}
            <div className="flex items-center justify-between mt-2 pt-3 border-t border-paper-deep/30">
              <span className="text-[11px] text-ink-ghost font-mono tabular-nums">
                {content.length > 0 ? `${content.length} 字` : "空"}
              </span>
              <div className="flex items-center gap-2">
                <button className="px-4 py-1.5 text-[12px] text-ink-faint hover:text-ink-soft rounded-lg hover:bg-paper-warm transition-all duration-200 cursor-pointer">
                  取消
                </button>
                <button className="px-4 py-1.5 text-[12px] text-cloud bg-bamboo hover:bg-bamboo-light rounded-lg transition-all duration-200 font-medium cursor-pointer shadow-[0_1px_3px_rgba(45,90,61,0.2)]">
                  保存
                </button>
              </div>
            </div>
          </div>
        ) : (
          <div className="p-2">
            {/* Note list in open mode */}
            <div className="space-y-0.5">
              {existingNotes.map((note) => (
                <button
                  key={note.id}
                  onMouseEnter={() => setHoveredNote(note.id)}
                  onMouseLeave={() => setHoveredNote(null)}
                  className="w-full text-left px-3.5 py-3 rounded-xl transition-all duration-200 cursor-pointer group hover:bg-paper-warm/70"
                >
                  <div className="flex items-baseline justify-between mb-0.5">
                    <span className="text-[13px] font-display font-medium text-ink-soft group-hover:text-ink transition-colors">
                      {note.title}
                    </span>
                    <span className="text-[11px] text-ink-ghost font-mono tabular-nums">
                      {note.date}
                    </span>
                  </div>
                  <p className="text-[12px] text-ink-ghost leading-relaxed line-clamp-1 group-hover:text-ink-faint transition-colors">
                    {note.preview}
                  </p>
                  {hoveredNote === note.id && (
                    <div className="mt-1.5 h-px bg-bamboo/10 transition-all duration-300" />
                  )}
                </button>
              ))}
            </div>
          </div>
        )}
      </div>

      {/* Label underneath */}
      <p className="text-[12px] text-ink-ghost text-center max-w-xs leading-relaxed">
        便签小窗 — 随时呼出，快速记录灵感
      </p>
    </div>
  );
}
