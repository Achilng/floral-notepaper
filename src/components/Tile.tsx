import type { CSSProperties, HTMLAttributes } from "react";
import {
  DEFAULT_TILE_COLOR,
  normalizeTileColor,
} from "../features/settings/tileColor";

export interface TileProps
  extends Omit<HTMLAttributes<HTMLDivElement>, "color" | "content" | "title"> {
  title?: string;
  content: string;
  color?: string;
  width?: number | string;
  rotation?: number;
}

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
  color = DEFAULT_TILE_COLOR,
  width = 260,
  rotation = 0,
  className = "",
  style,
  children,
  ...divProps
}: TileProps) {
  const tileColor = normalizeTileColor(color);
  const borderColor = colorWithAlpha(blendHex(tileColor, "#1a1a18", 0.18), 0.3);
  const cornerColor = colorWithAlpha(blendHex(tileColor, "#1a1a18", 0.3), 0.26);
  const mergedStyle: CSSProperties = {
    width,
    backgroundColor: tileColor,
    borderColor,
    transition: "box-shadow 0.3s ease",
    ...(rotation ? { transform: `rotate(${rotation}deg)` } : {}),
    ...style,
  };

  return (
    <div
      {...divProps}
      className={`relative rounded-xl border overflow-hidden select-none shadow-[0_1px_8px_rgba(26,26,24,0.04)] hover:shadow-[0_6px_24px_rgba(26,26,24,0.07)] ${className}`}
      style={mergedStyle}
    >
      <div className="px-4 pt-4 pb-4 h-full overflow-y-auto scrollbar-hidden">
        {title && (
          <div className="text-[15px] font-display text-ink-faint/45 tracking-wide mb-3 leading-snug">
            {title}
          </div>
        )}
        {content ? (
          <div className="text-[14px] leading-[1.8] text-ink-soft/80 whitespace-pre-wrap font-body">
            {content}
          </div>
        ) : (
          <div className="text-[14px] text-ink-ghost/40 font-body text-center py-6">
            空
          </div>
        )}
      </div>

      <CornerMarks color={cornerColor} />
      {children}
    </div>
  );
}

function hexToRgb(hexColor: string): [number, number, number] {
  const normalized = normalizeTileColor(hexColor).slice(1);
  return [
    Number.parseInt(normalized.slice(0, 2), 16),
    Number.parseInt(normalized.slice(2, 4), 16),
    Number.parseInt(normalized.slice(4, 6), 16),
  ];
}

function blendHex(source: string, target: string, amount: number): string {
  const [sourceR, sourceG, sourceB] = hexToRgb(source);
  const [targetR, targetG, targetB] = hexToRgb(target);
  const blend = (from: number, to: number) =>
    Math.round(from + (to - from) * amount)
      .toString(16)
      .padStart(2, "0");

  return `#${blend(sourceR, targetR)}${blend(sourceG, targetG)}${blend(sourceB, targetB)}`;
}

function colorWithAlpha(hexColor: string, alpha: number): string {
  const [red, green, blue] = hexToRgb(hexColor);
  return `rgba(${red}, ${green}, ${blue}, ${alpha})`;
}
