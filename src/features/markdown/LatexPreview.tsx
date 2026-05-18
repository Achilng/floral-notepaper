import { useMemo } from "react";
import katex from "katex";
import "katex/dist/katex.min.css";

interface LatexPreviewProps {
  content: string;
  fontSize?: number;
}

function renderLatexBlock(tex: string, displayMode: boolean): string {
  try {
    return katex.renderToString(tex, {
      displayMode,
      throwOnError: false,
      trust: true,
      strict: false,
    });
  } catch {
    return `<span class="text-red-400">${tex}</span>`;
  }
}

function parseLatexContent(content: string): Array<{ type: "text" | "block" | "inline"; value: string }> {
  const parts: Array<{ type: "text" | "block" | "inline"; value: string }> = [];
  let remaining = content;

  while (remaining.length > 0) {
    const blockStart = remaining.indexOf("$$");
    const inlineStart = remaining.indexOf("$");

    if (blockStart === -1 && inlineStart === -1) {
      if (remaining) parts.push({ type: "text", value: remaining });
      break;
    }

    if (blockStart !== -1 && (inlineStart === -1 || blockStart <= inlineStart)) {
      if (blockStart > 0) {
        parts.push({ type: "text", value: remaining.slice(0, blockStart) });
      }
      const blockEnd = remaining.indexOf("$$", blockStart + 2);
      if (blockEnd === -1) {
        parts.push({ type: "text", value: remaining });
        break;
      }
      parts.push({
        type: "block",
        value: remaining.slice(blockStart + 2, blockEnd),
      });
      remaining = remaining.slice(blockEnd + 2);
    } else {
      if (inlineStart > 0) {
        parts.push({ type: "text", value: remaining.slice(0, inlineStart) });
      }
      const inlineEnd = remaining.indexOf("$", inlineStart + 1);
      if (inlineEnd === -1) {
        parts.push({ type: "text", value: remaining });
        break;
      }
      parts.push({
        type: "inline",
        value: remaining.slice(inlineStart + 1, inlineEnd),
      });
      remaining = remaining.slice(inlineEnd + 1);
    }
  }

  return parts;
}

export function LatexPreview({ content, fontSize = 14 }: LatexPreviewProps) {
  const rendered = useMemo(() => parseLatexContent(content), [content]);

  if (!content.trim()) {
    return (
      <div className="max-w-[560px] font-body" style={{ fontSize: `${fontSize}px` }}>
        <p className="text-ink-ghost leading-[1.9]">LaTeX 预览区</p>
      </div>
    );
  }

  return (
    <div className="max-w-[560px] font-body" style={{ fontSize: `${fontSize}px` }}>
      {rendered.map((part, i) => {
        if (part.type === "text") {
          return (
            <span key={i} className="whitespace-pre-wrap text-ink-soft">
              {part.value}
            </span>
          );
        }
        if (part.type === "block") {
          return (
            <div
              key={i}
              className="my-4 py-3 px-4 rounded bg-paper-warm/50 overflow-x-auto text-center"
              dangerouslySetInnerHTML={{ __html: renderLatexBlock(part.value, true) }}
            />
          );
        }
        return (
          <span
            key={i}
            dangerouslySetInnerHTML={{ __html: renderLatexBlock(part.value, false) }}
          />
        );
      })}
    </div>
  );
}
