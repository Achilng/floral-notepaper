import { useState } from "react";

interface TileData {
  id: number;
  title: string;
  content: string;
  color: "default" | "warm" | "green" | "mist";
  width: number;
  rotation: number;
}

const tiles: TileData[] = [
  {
    id: 1,
    title: "读书笔记",
    content:
      "满地都是六便士，\n他却抬头看见了月亮。\n\n每个人都在忙碌地活着，\n却很少有人真正在生活。",
    color: "default",
    width: 260,
    rotation: -1.5,
  },
  {
    id: 2,
    title: "周末清单",
    content: "☐ 去菜市场\n☐ 整理书架\n☐ 给阳台的薄荷浇水\n☑ 写完项目文档",
    color: "warm",
    width: 220,
    rotation: 2,
  },
  {
    id: 3,
    title: "",
    content: "试试用 WebGL 做一个\n粒子书法效果？\n\n参考：rain.js + simplex noise",
    color: "green",
    width: 240,
    rotation: -0.8,
  },
  {
    id: 4,
    title: "灵感碎片",
    content: "花笺：轻如纸，\n随手记，随处贴。\n\n— 产品理念",
    color: "mist",
    width: 200,
    rotation: 1.2,
  },
];

const colorMap = {
  default: {
    bg: "bg-cloud",
    border: "border-paper-deep/30",
    accent: "bg-ink-ghost/30",
    handle: "text-ink-ghost",
  },
  warm: {
    bg: "bg-[#faf6ef]",
    border: "border-[#e8dfd0]/50",
    accent: "bg-[#d4c5a8]/30",
    handle: "text-[#c4b898]",
  },
  green: {
    bg: "bg-bamboo-mist",
    border: "border-bamboo-glow/50",
    accent: "bg-bamboo/10",
    handle: "text-bamboo-light/40",
  },
  mist: {
    bg: "bg-[#f0f4f8]",
    border: "border-[#d8e2ec]/50",
    accent: "bg-[#b8cce0]/20",
    handle: "text-[#b0c4d8]",
  },
};

export function TileShowcase() {
  const [draggedId, setDraggedId] = useState<number | null>(null);
  const [hoveredId, setHoveredId] = useState<number | null>(null);

  const positions = [
    { left: "8%", top: "12%" },
    { left: "55%", top: "6%" },
    { left: "5%", top: "55%" },
    { left: "52%", top: "50%" },
  ];

  return (
    <div className="flex flex-col items-center gap-10 pt-4 w-full">
      {/* Title */}
      <div className="text-center">
        <p className="text-[12px] text-ink-ghost leading-relaxed">
          钉住便签 → 磁贴常驻桌面，随时查看与编辑
        </p>
      </div>

      {/* Tile canvas */}
      <div className="relative w-full max-w-3xl h-[460px]">
        {/* Desktop simulation area */}
        <div className="absolute inset-0 rounded-2xl bg-paper-warm/40 border border-paper-deep/20 overflow-hidden">
          <div
            className="absolute inset-0 opacity-[0.04]"
            style={{
              backgroundImage:
                "radial-gradient(circle, var(--color-stone) 0.5px, transparent 0.5px)",
              backgroundSize: "24px 24px",
            }}
          />
          <div className="absolute top-4 left-5 text-[11px] text-ink-ghost/40 font-mono tracking-widest">
            DESKTOP
          </div>
        </div>

        {/* Tiles */}
        {tiles.map((tile, i) => {
          const colors = colorMap[tile.color];
          const isHovered = hoveredId === tile.id;
          const isDragged = draggedId === tile.id;

          return (
            <div
              key={tile.id}
              className="absolute cursor-default select-none"
              style={{
                ...positions[i],
                width: tile.width,
                zIndex: isDragged ? 50 : isHovered ? 40 : 10 + i,
              }}
              onMouseEnter={() => setHoveredId(tile.id)}
              onMouseLeave={() => {
                setHoveredId(null);
                setDraggedId(null);
              }}
            >
              <div
                className={`
                  noise-bg rounded-xl ${colors.bg} border ${colors.border} overflow-hidden
                  transition-all duration-300 ease-out
                  ${isHovered ? "shadow-[0_8px_30px_rgba(26,26,24,0.1)]" : "shadow-[0_2px_12px_rgba(26,26,24,0.05)]"}
                `}
                style={{
                  transform: `rotate(${isDragged ? 0 : tile.rotation}deg) scale(${isDragged ? 1.03 : isHovered ? 1.01 : 1})`,
                  transition:
                    "transform 0.3s cubic-bezier(0.22,1,0.36,1), box-shadow 0.3s ease",
                }}
              >
                {/* Drag handle bar */}
                <div
                  className="flex items-center gap-1 px-3 pt-2.5 pb-1 cursor-grab active:cursor-grabbing"
                  onMouseDown={() => setDraggedId(tile.id)}
                  onMouseUp={() => setDraggedId(null)}
                >
                  {/* Grip dots */}
                  <div className={`flex gap-[3px] ${colors.handle}`}>
                    {[...Array(6)].map((_, j) => (
                      <div
                        key={j}
                        className="w-[3px] h-[3px] rounded-full bg-current opacity-50"
                        style={{ marginTop: j % 2 === 0 ? 0 : 4 }}
                      />
                    ))}
                  </div>

                  <div className="flex-1" />

                  {tile.title && (
                    <span className="text-[11px] font-display text-ink-faint/60 tracking-wide">
                      {tile.title}
                    </span>
                  )}

                  {/* Close dot */}
                  <div
                    className={`w-2.5 h-2.5 rounded-full ml-2 transition-all duration-200 ${
                      isHovered
                        ? "bg-red-300/60 hover:bg-red-400/80 cursor-pointer"
                        : colors.accent
                    }`}
                  />
                </div>

                {/* Content */}
                <div className="px-3.5 pt-1 pb-3">
                  <div className="text-[13px] leading-[1.7] text-ink-soft/80 whitespace-pre-wrap font-body">
                    {tile.content}
                  </div>
                </div>

                {/* Resize handle */}
                <div
                  className={`absolute bottom-0 right-0 w-5 h-5 cursor-se-resize transition-opacity duration-200 ${
                    isHovered ? "opacity-40" : "opacity-0"
                  }`}
                >
                  <svg
                    className="absolute bottom-1 right-1 text-ink-faint"
                    width="8"
                    height="8"
                    viewBox="0 0 8 8"
                    fill="currentColor"
                  >
                    <circle cx="6" cy="6" r="1" />
                    <circle cx="6" cy="2.5" r="0.8" opacity="0.5" />
                    <circle cx="2.5" cy="6" r="0.8" opacity="0.5" />
                  </svg>
                </div>
              </div>
            </div>
          );
        })}
      </div>

      {/* Mode indicator */}
      <div className="flex items-center gap-6 text-[11px] text-ink-ghost">
        <div className="flex items-center gap-2">
          <div className="w-1.5 h-1.5 rounded-full bg-bamboo/50" />
          <span>置顶悬浮</span>
        </div>
        <div className="flex items-center gap-2">
          <div className="w-1.5 h-1.5 rounded-full bg-stone/40" />
          <span>嵌入桌面</span>
        </div>
        <div className="flex items-center gap-2">
          <svg
            width="10"
            height="10"
            viewBox="0 0 10 10"
            className="text-ink-ghost/50"
          >
            <circle cx="5" cy="8" r="1" fill="currentColor" />
            <circle cx="8" cy="5" r="0.8" fill="currentColor" opacity="0.5" />
            <circle cx="5" cy="5" r="0.8" fill="currentColor" opacity="0.3" />
          </svg>
          <span>拖拽缩放</span>
        </div>
      </div>
    </div>
  );
}
