import type { Plugin } from "unified";
import type { Root } from "mdast";
import { visit } from "unist-util-visit";
import type { Text } from "mdast";

const HIGHLIGHT_MARKER = "==";

const remarkHighlight: Plugin<[], Root> = function () {
  return (tree) => {
    visit(tree, "text", (node: Text, index, parent) => {
      if (index == null || parent == null) return;
      if (!node.value.includes(HIGHLIGHT_MARKER)) return;

      const parts = splitHighlight(node.value);
      if (parts.length === 1) return;

      const newNodes = parts.map((part) => {
        if (part.highlight) {
          return {
            type: "html" as const,
            value: `<mark>${part.text}</mark>`,
          };
        }
        return {
          type: "text" as const,
          value: part.text,
        };
      });

      parent.children.splice(index, 1, ...newNodes);
    });
  };
};

interface TextPart {
  text: string;
  highlight: boolean;
}

function splitHighlight(value: string): TextPart[] {
  const parts: TextPart[] = [];
  let remaining = value;
  let isHighlight = false;

  while (remaining.length > 0) {
    const markerIndex = remaining.indexOf(HIGHLIGHT_MARKER);
    if (markerIndex === -1) {
      parts.push({ text: remaining, highlight: isHighlight });
      break;
    }

    const before = remaining.slice(0, markerIndex);
    if (before.length > 0) {
      parts.push({ text: before, highlight: isHighlight });
    }

    isHighlight = !isHighlight;
    remaining = remaining.slice(markerIndex + HIGHLIGHT_MARKER.length);
  }

  return parts;
}

export default remarkHighlight;
