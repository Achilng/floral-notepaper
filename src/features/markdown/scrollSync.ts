interface ParsedBlock {
  text: string;
  startLine: number; // inclusive
  endLine: number; // exclusive
}

export interface ScrollSyncMap {
  source: number[];
  target: number[];
}

const MIRROR_STYLE_PROPERTIES = [
  "font",
  "fontSize",
  "fontFamily",
  "fontWeight",
  "fontStyle",
  "lineHeight",
  "letterSpacing",
  "wordSpacing",
  "wordBreak",
  "overflowWrap",
  "wordWrap",
  "tabSize",
  "padding",
  "paddingTop",
  "paddingRight",
  "paddingBottom",
  "paddingLeft",
  "border",
  "borderTop",
  "borderRight",
  "borderBottom",
  "borderLeft",
] as const;

const MIRROR_BUILD_YIELD_INTERVAL = 500;
const MARKER_MEASURE_YIELD_INTERVAL = 64;

function yieldToPendingWork(): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, 0));
}

function isStandaloneLinkReferenceDefinition(line: string): boolean {
  return /^\s{0,3}\[[^\]]+]:\s*\S/.test(line);
}

/** Parse markdown into non-empty logical blocks with line boundaries. */
function parseBlocks(text: string): ParsedBlock[] {
  const lines = text.split("\n");
  const blocks: ParsedBlock[] = [];
  let i = 0;

  const push = (start: number, end: number) => {
    blocks.push({ text: lines.slice(start, end).join("\n"), startLine: start, endLine: end });
  };

  while (i < lines.length) {
    if (lines[i].trim() === "") {
      i++;
      continue;
    }

    const line = lines[i];
    const trimmed = line.trim();

    // 链接引用定义在预览中会变成不可见的 definition 节点，需跳过。
    if (isStandaloneLinkReferenceDefinition(line)) {
      i++;
      continue;
    }

    // Fenced code block
    const fenceMatch = trimmed.match(/^(```|~~~)/);
    if (fenceMatch) {
      const fence = fenceMatch[1];
      const start = i;
      i++;
      while (i < lines.length && !lines[i].trim().startsWith(fence)) i++;
      if (i < lines.length) i++;
      push(start, i);
      continue;
    }

    // Display math block
    if (trimmed === "$$") {
      const start = i;
      i++;
      while (i < lines.length && lines[i].trim() !== "$$") i++;
      if (i < lines.length) i++;
      push(start, i);
      continue;
    }

    // Heading / HR
    if (/^#{1,6}\s/.test(line) || /^(-{3,}|\*{3,}|_{3,})\s*$/.test(trimmed)) {
      push(i, i + 1);
      i++;
      continue;
    }

    // Blockquote
    if (line.startsWith(">")) {
      const start = i;
      while (i < lines.length && lines[i].startsWith(">")) i++;
      push(start, i);
      continue;
    }

    // Table
    if (/^\s*\|/.test(line)) {
      const start = i;
      while (i < lines.length && /^\s*\|/.test(lines[i])) i++;
      push(start, i);
      continue;
    }

    // List
    if (/^(\s*[-*+]\s|\s*\d+[.)]\s)/.test(line)) {
      const start = i;
      while (i < lines.length) {
        const l = lines[i];
        if (l.trim() === "") {
          const next = i + 1;
          if (next < lines.length && /^(\s*[-*+]\s|\s*\d+[.)]\s)/.test(lines[next])) {
            i++;
            continue;
          }
          break;
        }
        if (/^(#{1,6}\s|```|~~~|>)/.test(l) || l.trim() === "$$") break;
        i++;
      }
      push(start, i);
      continue;
    }

    // Paragraph
    const start = i;
    while (i < lines.length && lines[i].trim() !== "") {
      const l = lines[i];
      if (/^(#{1,6}\s|```|~~~|>)/.test(l)) break;
      if (l.trim() === "$$") break;
      if (/^(-{3,}|\*{3,}|_{3,})\s*$/.test(l.trim())) break;
      i++;
    }
    if (i > start) push(start, i);
  }

  return blocks;
}

/** 测量每个块在 textarea 中的 scrollTop 偏移量，支持 AbortSignal 取消。 */
export async function measureBlockOffsets(
  content: string,
  sourceTextarea: HTMLTextAreaElement,
  signal?: AbortSignal,
): Promise<number[]> {
  const blocks = parseBlocks(content);
  if (blocks.length === 0) return [];

  const style = getComputedStyle(sourceTextarea);
  const paddingTop = parseFloat(style.paddingTop) || 0;
  const measure = document.createElement("div");
  const width =
    style.width && style.width !== "auto" ? style.width : `${sourceTextarea.clientWidth}px`;
  const markers: HTMLElement[] = [];

  measure.style.cssText = `
    position: fixed; top: -9999px; left: -9999px; visibility: hidden;
    width: ${width};
    height: auto;
    margin: 0;
    box-sizing: ${style.boxSizing};
    white-space: pre-wrap;
    overflow: hidden;
    pointer-events: none;
    contain: layout style;
    ${MIRROR_STYLE_PROPERTIES.map((property) => {
      const cssName = property.replace(/[A-Z]/g, (letter) => `-${letter.toLowerCase()}`);
      return `${cssName}: ${style[property]};`;
    }).join("\n    ")}
  `;

  const markerLines = new Map<number, number>();
  blocks.forEach((block, index) => markerLines.set(block.startLine, index));

  const lines = content.split("\n");
  for (let lineIndex = 0; lineIndex < lines.length; lineIndex++) {
    const line = lines[lineIndex];
    const markerIndex = markerLines.get(lineIndex);
    if (markerIndex !== undefined) {
      const marker = document.createElement("span");
      marker.style.cssText =
        "display: inline-block; width: 0; height: 0; overflow: hidden; line-height: 0; font-size: 0; vertical-align: top;";
      markers[markerIndex] = marker;
      measure.appendChild(marker);
    }
    measure.appendChild(document.createTextNode(line));
    if (lineIndex < lines.length - 1) measure.appendChild(document.createTextNode("\n"));

    if ((lineIndex + 1) % MIRROR_BUILD_YIELD_INTERVAL === 0) {
      await yieldToPendingWork();
      if (signal?.aborted) return [];
    }
  }

  if (signal?.aborted) return [];

  document.body.appendChild(measure);

  const offsets: number[] = [];
  try {
    for (let index = 0; index < markers.length; index++) {
      const marker = markers[index];
      if (signal?.aborted) break;
      offsets.push(Math.max(0, marker.offsetTop - paddingTop));
      if (signal?.aborted) break;

      if ((index + 1) % MARKER_MEASURE_YIELD_INTERVAL === 0) {
        await yieldToPendingWork();
      }
    }
  } finally {
    document.body.removeChild(measure);
  }

  return offsets;
}

function clamp(value: number, min: number, max: number): number {
  return Math.min(Math.max(value, min), max);
}

/**
 * Build a compact, monotonic mapping between two scrollable surfaces.
 * The first and last scroll positions are always paired so both panes
 * remain aligned at the document boundaries.
 */
export function createScrollSyncMap(
  sourceOffsets: number[],
  targetOffsets: number[],
  sourceMaxScroll: number,
  targetMaxScroll: number,
): ScrollSyncMap {
  const sourceMax = Math.max(0, sourceMaxScroll);
  const targetMax = Math.max(0, targetMaxScroll);
  const source = [0];
  const target = [0];
  const count = Math.min(sourceOffsets.length, targetOffsets.length);

  for (let index = 0; index < count; index++) {
    const rawSource = sourceOffsets[index];
    const rawTarget = targetOffsets[index];
    if (!Number.isFinite(rawSource) || !Number.isFinite(rawTarget)) continue;

    const nextSource = clamp(rawSource, 0, sourceMax);
    if (nextSource <= 0 || nextSource >= sourceMax) continue;

    const previousSource = source[source.length - 1];
    const nextTarget = Math.max(target[target.length - 1], clamp(rawTarget, 0, targetMax));
    if (nextSource <= previousSource) {
      if (nextSource === previousSource) target[target.length - 1] = nextTarget;
      continue;
    }

    source.push(nextSource);
    target.push(nextTarget);
  }

  if (sourceMax > 0) {
    source.push(sourceMax);
    target.push(targetMax);
  }

  return { source, target };
}

/** Map a scroll position through precomputed anchors using linear interpolation. */
export function interpolateScrollOffset(map: ScrollSyncMap, scrollTop: number): number {
  const { source, target } = map;
  if (source.length < 2) return 0;

  const maxSource = source[source.length - 1];
  const position = clamp(scrollTop, 0, maxSource);
  if (position <= 0) return 0;
  if (position >= maxSource) return target[target.length - 1];

  let low = 0;
  let high = source.length - 1;
  while (low + 1 < high) {
    const middle = Math.floor((low + high) / 2);
    if (source[middle] <= position) low = middle;
    else high = middle;
  }

  const sourceRange = source[high] - source[low];
  if (sourceRange <= 0) return target[high];
  const progress = (position - source[low]) / sourceRange;
  return target[low] + (target[high] - target[low]) * progress;
}

/**
 * Add data-block-index attributes to block-level children
 * of the MarkdownPreview root element (.font-body).
 * Indices match the non-empty block indices from parseBlocks.
 */
export function tagPreviewBlocks(container: HTMLElement): void {
  const root = container.querySelector<HTMLElement>(".font-body");
  if (!root) return;
  let index = 0;
  for (const child of root.children) {
    if (child instanceof HTMLElement) {
      child.setAttribute("data-block-index", String(index++));
    }
  }
}

/** Measure tagged preview blocks in the scroll container's coordinate space. */
export function measurePreviewBlockOffsets(container: HTMLElement): number[] {
  const containerRect = container.getBoundingClientRect();
  const elements = container.querySelectorAll<HTMLElement>("[data-block-index]");
  const offsets: number[] = [];

  for (const element of elements) {
    const index = Number.parseInt(element.dataset.blockIndex ?? "", 10);
    if (!Number.isFinite(index)) continue;
    const elementRect = element.getBoundingClientRect();
    offsets[index] = Math.max(
      0,
      container.scrollTop + elementRect.top - containerRect.top - container.clientTop,
    );
  }

  return offsets;
}
