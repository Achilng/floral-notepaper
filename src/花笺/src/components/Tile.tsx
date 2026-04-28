import { useState } from "react";
import type { CSSProperties, HTMLAttributes } from "react";

export type TileColor = "default" | "warm" | "green" | "mist" | "cyan";

export interface TileProps
  extends Omit<HTMLAttributes<HTMLDivElement>, "color" | "content" | "title"> {
  title?: string;
  content: string;
  color?: TileColor;
  width?: number | string;
  rotation?: number;
}

const colorMap: Record<
  TileColor,
  { bg: string; border: string; corner: string }
> = {
  default: {
    bg: "bg-cloud",
    border: "border-paper-deep/30",
    corner: "rgba(184, 184, 174, 0.18)",
  },
  warm: {
    bg: "bg-[#faf6ef]",
    border: "border-[#e8dfd0]/40",
    corner: "rgba(196, 184, 152, 0.22)",
  },
  green: {
    bg: "bg-bamboo-mist",
    border: "border-bamboo-glow/40",
    corner: "rgba(58, 122, 82, 0.18)",
  },
  cyan: {
    bg: "bg-[#d8eee9]",
    border: "border-[#99cbc1]/55",
    corner: "rgba(24, 104, 99, 0.26)",
  },
  mist: {
    bg: "bg-[#f0f4f8]",
    border: "border-[#d8e2ec]/40",
    corner: "rgba(176, 196, 216, 0.22)",
  },
};

const MARK_SIZE = 8;
const MARK_OFFSET = 6;

const cornerPaths = [
  {
    pos: { top: MARK_OFFSET, left: MARK_OFFSET },
    d: `M0,${MARK_SIZE} L0,0 L${MARK_SIZE},0`,
  },
  {
    pos: { top: MARK_OFFSET, right: MARK_OFFSET },
    d: `M0,0 L${MARK_SIZE},0 L${MARK_SIZE},${MARK_SIZE}`,
  },
  {
    pos: { bottom: MARK_OFFSET, left: MARK_OFFSET },
    d: `M0,0 L0,${MARK_SIZE} L${MARK_SIZE},${MARK_SIZE}`,
  },
  {
    pos: { bottom: MARK_OFFSET, right: MARK_OFFSET },
    d: `M${MARK_SIZE},0 L${MARK_SIZE},${MARK_SIZE} L0,${MARK_SIZE}`,
  },
];

function CornerMarks({ color }: { color: string }) {
  return (
    <>
      {cornerPaths.map((mark, index) => (
        <svg
          key={index}
          className="absolute pointer-events-none"
          data-tile-corner-mark="true"
          style={mark.pos as CSSProperties}
          width={MARK_SIZE}
          height={MARK_SIZE}
          viewBox={`0 0 ${MARK_SIZE} ${MARK_SIZE}`}
        >
          <path
            d={mark.d}
            stroke={color}
            strokeWidth="0.8"
            fill="none"
            strokeLinecap="round"
            strokeLinejoin="round"
          />
        </svg>
      ))}
    </>
  );
}

export function Tile({
  title,
  content,
  color = "default",
  width = 260,
  rotation = 0,
  className = "",
  style,
  children,
  ...divProps
}: TileProps) {
  const [isHovered, setIsHovered] = useState(false);
  const scheme = colorMap[color];
  const mergedStyle: CSSProperties = {
    width,
    transform: `rotate(${rotation}deg) scale(${isHovered ? 1.008 : 1})`,
    transition:
      "transform 0.3s cubic-bezier(0.22,1,0.36,1), box-shadow 0.3s ease",
    ...style,
  };

  return (
    <div
      {...divProps}
      className={`relative rounded-xl ${scheme.bg} border ${scheme.border} overflow-hidden select-none ${
        isHovered
          ? "shadow-[0_6px_24px_rgba(26,26,24,0.07)]"
          : "shadow-[0_1px_8px_rgba(26,26,24,0.04)]"
      } hover:shadow-[0_6px_24px_rgba(26,26,24,0.07)] hover:scale-[1.008] ${className}`}
      style={mergedStyle}
      onMouseEnter={(event) => {
        setIsHovered(true);
        divProps.onMouseEnter?.(event);
      }}
      onMouseLeave={(event) => {
        setIsHovered(false);
        divProps.onMouseLeave?.(event);
      }}
    >
      <div className="px-4 pt-4 pb-4">
        {title && (
          <div className="text-[11px] font-display text-ink-faint/45 tracking-wide mb-2 leading-none">
            {title}
          </div>
        )}
        {content ? (
          <div className="text-[13px] leading-[1.8] text-ink-soft/75 whitespace-pre-wrap font-body">
            {content}
          </div>
        ) : (
          <div className="text-[13px] text-ink-ghost/40 font-body text-center py-6">
            空
          </div>
        )}
      </div>

      <CornerMarks color={scheme.corner} />
      {children}
    </div>
  );
}
