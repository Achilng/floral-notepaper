import { useState } from "react";

interface NoteItem {
  id: number;
  title: string;
  preview: string;
  date: string;
  updatedTime: string;
  wordCount: number;
}

const mockNotes: NoteItem[] = [
  {
    id: 1,
    title: "月亮与六便士",
    preview: "满地都是六便士，他却抬头看见了月亮。每个人都在忙碌地活着……",
    date: "04-27",
    updatedTime: "14:32",
    wordCount: 1240,
  },
  {
    id: 2,
    title: "花笺产品理念",
    preview: "轻如纸，随手记，随处贴。所有的笔记都应该是透明的 .md 文件……",
    date: "04-26",
    updatedTime: "22:15",
    wordCount: 680,
  },
  {
    id: 3,
    title: "",
    preview: "试试用 WebGL 做一个粒子书法效果？参考 rain.js + simplex noise",
    date: "04-25",
    updatedTime: "09:41",
    wordCount: 156,
  },
  {
    id: 4,
    title: "周末采购清单",
    preview: "牛奶、面包、橄榄油、番茄、鸡蛋、酱油",
    date: "04-24",
    updatedTime: "18:03",
    wordCount: 42,
  },
  {
    id: 5,
    title: "给自己的信",
    preview: "不要急，慢慢来。所有的经历都会变成养分，所有的等待都会有意义……",
    date: "04-22",
    updatedTime: "23:58",
    wordCount: 892,
  },
  {
    id: 6,
    title: "Rust 学习笔记",
    preview: "所有权系统是 Rust 最独特的特性。每个值都有一个所有者……",
    date: "04-20",
    updatedTime: "16:20",
    wordCount: 2130,
  },
  {
    id: 7,
    title: "读《人生的智慧》",
    preview: "叔本华说：人要么孤独，要么庸俗。独处时不觉得无聊的人……",
    date: "04-18",
    updatedTime: "11:45",
    wordCount: 1560,
  },
];

const sampleMarkdown = `满地都是六便士，他却抬头看见了月亮。

每个人都在忙碌地活着，却很少有人真正在**生活**。毛姆笔下的斯特里克兰德抛弃了一切世俗意义上的成功，去追寻内心深处那不可名状的渴望。

## 关于选择

> 做自己最想做的事，过自己想过的生活，心平气和，怎么能叫作践自己？做一个有名的外科医生，一年赚一万英镑，娶一位漂亮的妻子，就是成功？我想，这取决于你如何看待成功的意义。

这段话让我想了很久。我们总是在别人的评价体系里寻找自己的位置，却忘了问自己——

1. 什么事情让你忘记时间？
2. 什么事情即使没有报酬你也愿意做？
3. 什么事情让你感到​*活着*​？

## 摘录

- 「我用尽了全力，过着平凡的一生。」
- 「追逐梦想就是追逐自己的厄运。」
- 「在满地都是六便士的街上，他抬起头看到了月光。」

---

*2026年4月27日，读完第三遍后记*`;

type ViewMode = "edit" | "preview" | "split";

export function MainWindow() {
  const [selectedId, setSelectedId] = useState(1);
  const [searchQuery, setSearchQuery] = useState("");
  const [viewMode, setViewMode] = useState<ViewMode>("split");
  const [sidebarCollapsed, setSidebarCollapsed] = useState(false);
  const [content, setContent] = useState(sampleMarkdown);
  const [title, setTitle] = useState("月亮与六便士");
  const [isSaved, setIsSaved] = useState(true);
  const [hoveredId, setHoveredId] = useState<number | null>(null);

  const selectedNote = mockNotes.find((n) => n.id === selectedId);

  const filteredNotes = mockNotes.filter((n) => {
    if (!searchQuery) return true;
    const displayTitle = n.title || n.preview.slice(0, 20);
    return (
      displayTitle.toLowerCase().includes(searchQuery.toLowerCase()) ||
      n.preview.toLowerCase().includes(searchQuery.toLowerCase())
    );
  });

  const handleContentChange = (value: string) => {
    setContent(value);
    setIsSaved(false);
  };

  const renderMarkdownPreview = (md: string) => {
    const lines = md.split("\n");
    return lines.map((line, i) => {
      if (line.startsWith("## ")) {
        return (
          <h2
            key={i}
            className="text-[17px] font-display font-bold text-ink mt-7 mb-3 tracking-wide"
          >
            {line.slice(3)}
          </h2>
        );
      }
      if (line.startsWith("# ")) {
        return (
          <h1
            key={i}
            className="text-[22px] font-display font-bold text-ink mt-6 mb-4 tracking-wide"
          >
            {line.slice(2)}
          </h1>
        );
      }
      if (line.startsWith("> ")) {
        return (
          <blockquote
            key={i}
            className="border-l-2 border-bamboo/40 pl-4 my-3 text-ink-soft/80 italic leading-[1.9]"
          >
            {renderInlineMarkdown(line.slice(2))}
          </blockquote>
        );
      }
      if (line.startsWith("- ")) {
        return (
          <li
            key={i}
            className="ml-4 text-ink-soft leading-[1.9] list-disc list-outside marker:text-bamboo/40"
          >
            {renderInlineMarkdown(line.slice(2))}
          </li>
        );
      }
      if (/^\d+\.\s/.test(line)) {
        const text = line.replace(/^\d+\.\s/, "");
        return (
          <li
            key={i}
            className="ml-4 text-ink-soft leading-[1.9] list-decimal list-outside marker:text-bamboo/50 marker:font-mono marker:text-[12px]"
          >
            {renderInlineMarkdown(text)}
          </li>
        );
      }
      if (line === "---") {
        return (
          <hr
            key={i}
            className="my-6 border-none h-px bg-gradient-to-r from-transparent via-paper-deep to-transparent"
          />
        );
      }
      if (line.trim() === "") {
        return <div key={i} className="h-3" />;
      }
      return (
        <p key={i} className="text-ink-soft leading-[1.9]">
          {renderInlineMarkdown(line)}
        </p>
      );
    });
  };

  const renderInlineMarkdown = (text: string) => {
    const parts = text.split(/(\*\*[^*]+\*\*|\*[^*]+\*|`[^`]+`|「[^」]+」)/g);
    return parts.map((part, i) => {
      if (part.startsWith("**") && part.endsWith("**")) {
        return (
          <strong key={i} className="font-semibold text-ink">
            {part.slice(2, -2)}
          </strong>
        );
      }
      if (part.startsWith("*") && part.endsWith("*")) {
        return (
          <em key={i} className="italic text-bamboo-light">
            {part.slice(1, -1)}
          </em>
        );
      }
      if (part.startsWith("`") && part.endsWith("`")) {
        return (
          <code
            key={i}
            className="px-1.5 py-0.5 text-[12px] font-mono bg-paper-warm rounded text-bamboo"
          >
            {part.slice(1, -1)}
          </code>
        );
      }
      if (part.startsWith("「") && part.endsWith("」")) {
        return (
          <span key={i} className="text-ink font-display">
            {part}
          </span>
        );
      }
      return <span key={i}>{part}</span>;
    });
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
        {/* Custom title bar */}
        <div className="flex items-center justify-between px-4 h-11 bg-paper/60 border-b border-paper-deep/30 shrink-0 select-none">
          <div className="flex items-center gap-3">
            {/* Window controls */}
            <div className="flex items-center gap-[7px]">
              <div className="w-[11px] h-[11px] rounded-full bg-[#ff5f57] border border-[#e14640]/40 hover:brightness-90 cursor-pointer transition-all" />
              <div className="w-[11px] h-[11px] rounded-full bg-[#febc2e] border border-[#d8a118]/40 hover:brightness-90 cursor-pointer transition-all" />
              <div className="w-[11px] h-[11px] rounded-full bg-[#28c840] border border-[#1aab2e]/40 hover:brightness-90 cursor-pointer transition-all" />
            </div>
            <div className="h-4 w-px bg-paper-deep/40" />
            <span className="text-[13px] font-display font-medium text-ink-soft tracking-wide">
              花笺
            </span>
            <span className="text-[11px] text-ink-ghost font-body">—</span>
            <span className="text-[11px] text-ink-faint font-body truncate max-w-[200px]">
              {title || "无标题笔记"}
            </span>
          </div>
          <div className="flex items-center gap-1">
            {/* Settings gear */}
            <button className="w-7 h-7 flex items-center justify-center rounded-lg text-ink-ghost hover:text-ink-faint hover:bg-paper-warm transition-all cursor-pointer">
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

        {/* Main content area */}
        <div className="flex flex-1 min-h-0">
          {/* Sidebar */}
          <div
            className={`border-r border-paper-deep/30 bg-paper/40 flex flex-col shrink-0 transition-all duration-300 ${
              sidebarCollapsed ? "w-0 overflow-hidden" : "w-[280px]"
            }`}
          >
            {/* Sidebar header */}
            <div className="px-3 pt-3 pb-2 shrink-0">
              {/* Search bar */}
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
                  onChange={(e) => setSearchQuery(e.target.value)}
                  placeholder="搜索笔记…"
                  className="flex-1 text-[12px] font-body text-ink placeholder:text-ink-ghost/60 bg-transparent"
                />
                {searchQuery && (
                  <button
                    onClick={() => setSearchQuery("")}
                    className="text-ink-ghost hover:text-ink-faint transition-colors cursor-pointer"
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

            {/* New note button */}
            <div className="px-3 pb-2 shrink-0">
              <button className="w-full flex items-center gap-2 px-2.5 py-1.5 rounded-lg text-[12px] font-body text-bamboo hover:bg-bamboo-mist/60 transition-all cursor-pointer group">
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

            {/* Note count */}
            <div className="px-5 pb-1.5 shrink-0">
              <span className="text-[10px] text-ink-ghost font-mono tracking-wider uppercase">
                {filteredNotes.length} 篇笔记
              </span>
            </div>

            {/* Note list */}
            <div className="flex-1 overflow-y-auto px-2 pb-2">
              <div className="space-y-0.5">
                {filteredNotes.map((note) => {
                  const isSelected = note.id === selectedId;
                  const isHovered = note.id === hoveredId;
                  const displayTitle =
                    note.title || note.preview.slice(0, 20) + "…";

                  return (
                    <button
                      key={note.id}
                      onClick={() => {
                        setSelectedId(note.id);
                        setTitle(note.title);
                        setIsSaved(true);
                      }}
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
                      {/* Active indicator */}
                      {isSelected && (
                        <div className="absolute left-0 top-1/2 -translate-y-1/2 w-[3px] h-5 rounded-r-full bg-bamboo/60" />
                      )}

                      <div className="flex items-baseline justify-between mb-0.5">
                        <span
                          className={`text-[13px] font-display font-medium truncate pr-2 transition-colors ${
                            isSelected ? "text-bamboo" : "text-ink-soft"
                          }`}
                        >
                          {displayTitle}
                        </span>
                        <span className="text-[10px] text-ink-ghost font-mono tabular-nums shrink-0">
                          {note.date}
                        </span>
                      </div>

                      <p className="text-[11px] text-ink-ghost leading-relaxed line-clamp-2 group-hover:text-ink-faint transition-colors">
                        {note.preview}
                      </p>

                      <div className="flex items-center gap-2 mt-1">
                        <span className="text-[10px] text-ink-ghost/60 font-mono tabular-nums">
                          {note.updatedTime}
                        </span>
                        <span className="text-[10px] text-ink-ghost/40">·</span>
                        <span className="text-[10px] text-ink-ghost/60 font-mono tabular-nums">
                          {note.wordCount} 字
                        </span>
                      </div>
                    </button>
                  );
                })}
              </div>
            </div>
          </div>

          {/* Editor area */}
          <div className="flex-1 flex flex-col min-w-0">
            {/* Editor toolbar */}
            <div className="flex items-center justify-between px-4 h-10 border-b border-paper-deep/20 shrink-0 bg-paper/20">
              <div className="flex items-center gap-1">
                {/* Sidebar toggle */}
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

                {/* Pin to tile */}
                <button
                  className="w-7 h-7 flex items-center justify-center rounded-lg text-ink-ghost hover:text-bamboo hover:bg-bamboo-mist/50 transition-all cursor-pointer"
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

                {/* Export */}
                <button
                  className="w-7 h-7 flex items-center justify-center rounded-lg text-ink-ghost hover:text-ink-faint hover:bg-paper-warm transition-all cursor-pointer"
                  title="导出"
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
                    <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4" />
                    <polyline points="7,10 12,15 17,10" />
                    <line x1="12" y1="15" x2="12" y2="3" />
                  </svg>
                </button>

                {/* Delete */}
                <button
                  className="w-7 h-7 flex items-center justify-center rounded-lg text-ink-ghost hover:text-red-400 hover:bg-red-50 transition-all cursor-pointer"
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

              {/* View mode toggle */}
              <div className="flex items-center bg-paper-warm/60 rounded-lg p-[2px] border border-paper-deep/30">
                <button
                  onClick={() => setViewMode("edit")}
                  className={`px-3 py-1 rounded-md text-[11px] transition-all duration-200 cursor-pointer ${
                    viewMode === "edit"
                      ? "bg-cloud text-bamboo font-medium shadow-[0_1px_2px_rgba(0,0,0,0.04)]"
                      : "text-ink-ghost hover:text-ink-faint"
                  }`}
                >
                  编辑
                </button>
                <button
                  onClick={() => setViewMode("split")}
                  className={`px-3 py-1 rounded-md text-[11px] transition-all duration-200 cursor-pointer ${
                    viewMode === "split"
                      ? "bg-cloud text-bamboo font-medium shadow-[0_1px_2px_rgba(0,0,0,0.04)]"
                      : "text-ink-ghost hover:text-ink-faint"
                  }`}
                >
                  分栏
                </button>
                <button
                  onClick={() => setViewMode("preview")}
                  className={`px-3 py-1 rounded-md text-[11px] transition-all duration-200 cursor-pointer ${
                    viewMode === "preview"
                      ? "bg-cloud text-bamboo font-medium shadow-[0_1px_2px_rgba(0,0,0,0.04)]"
                      : "text-ink-ghost hover:text-ink-faint"
                  }`}
                >
                  预览
                </button>
              </div>
            </div>

            {/* Title area */}
            <div className="px-6 pt-4 pb-2 shrink-0 border-b border-paper-deep/15">
              <input
                type="text"
                value={title}
                onChange={(e) => {
                  setTitle(e.target.value);
                  setIsSaved(false);
                }}
                placeholder="无标题笔记"
                className="w-full text-[20px] font-display font-bold text-ink placeholder:text-ink-ghost/50 tracking-wide"
              />
              <div className="flex items-center gap-3 mt-1.5">
                <span className="text-[10px] text-ink-ghost font-mono tabular-nums">
                  {selectedNote?.date} {selectedNote?.updatedTime}
                </span>
                <span className="text-[10px] text-ink-ghost/40">·</span>
                <span className="text-[10px] text-ink-ghost font-mono tabular-nums">
                  {content.length} 字
                </span>
                <span className="text-[10px] text-ink-ghost/40">·</span>
                <span
                  className={`text-[10px] font-mono tabular-nums ${isSaved ? "text-bamboo/60" : "text-amber-500/70"}`}
                >
                  {isSaved ? "已保存" : "未保存"}
                </span>
              </div>
            </div>

            {/* Editor / Preview panes */}
            <div className="flex-1 flex min-h-0">
              {/* Editor pane */}
              {(viewMode === "edit" || viewMode === "split") && (
                <div
                  className={`flex flex-col min-h-0 ${viewMode === "split" ? "w-1/2 border-r border-paper-deep/20" : "w-full"}`}
                >
                  {/* Markdown format hints */}
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
                    ].map((btn) => (
                      <button
                        key={btn.label}
                        title={btn.title}
                        className={`w-6 h-6 flex items-center justify-center rounded text-[11px] text-ink-ghost hover:text-ink-faint hover:bg-paper-warm transition-all cursor-pointer ${btn.style}`}
                      >
                        {btn.label}
                      </button>
                    ))}
                  </div>

                  <div className="flex-1 overflow-y-auto px-5 pb-4">
                    <textarea
                      value={content}
                      onChange={(e) => handleContentChange(e.target.value)}
                      className="w-full h-full text-[13.5px] leading-[1.9] text-ink-soft font-mono placeholder:text-ink-ghost/40"
                      placeholder="开始写作……"
                      spellCheck={false}
                    />
                  </div>
                </div>
              )}

              {/* Preview pane */}
              {(viewMode === "preview" || viewMode === "split") && (
                <div
                  className={`flex flex-col min-h-0 ${viewMode === "split" ? "w-1/2" : "w-full"}`}
                >
                  {viewMode === "split" && (
                    <div className="px-4 pt-2.5 pb-1 shrink-0">
                      <span className="text-[10px] text-ink-ghost/60 font-mono tracking-widest uppercase">
                        Preview
                      </span>
                    </div>
                  )}
                  <div
                    className={`flex-1 overflow-y-auto px-6 pb-6 ${viewMode === "preview" ? "pt-3" : "pt-1"}`}
                  >
                    <div className="max-w-[560px] text-[14px] font-body">
                      {renderMarkdownPreview(content)}
                    </div>
                  </div>
                </div>
              )}
            </div>

            {/* Status bar */}
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
                  {(new TextEncoder().encode(content).length / 1024).toFixed(1)}{" "}
                  KB
                </span>
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}
