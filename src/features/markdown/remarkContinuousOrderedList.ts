import type { Root, List } from "mdast";
import type { Plugin } from "unified";

/**
 * remark plugin to fix consecutive ordered list numbering.
 *
 * In CommonMark/GFM, ordered lists separated by blank lines or other content
 * are parsed as independent <ol> elements, each starting from 1. This plugin
 * traverses the AST and sets the correct `start` attribute on subsequent
 * ordered lists so that numbering continues sequentially across "soft" breaks
 * (paragraphs, unordered lists), but resets on "structural" breaks
 * (headings, thematic breaks).
 *
 * Fixes: https://github.com/Achilng/floral-notepaper/issues/321
 */

/** Node types that reset numbering — they signal a new logical section. */
const SECTION_BREAK_TYPES = new Set(["heading", "thematicBreak"]);

const remarkContinuousOrderedList: Plugin<[], Root> = () => (tree) => {
  fixOrderedLists(tree.children);
};

function fixOrderedLists(children: Root["children"]): void {
  let nextStart = 0; // 0 = no tracked numbering yet
  let justReset = false; // true after a section break

  for (const node of children) {
    if (node.type === "list" && node.ordered) {
      const list = node as List;
      if (justReset) {
        // After a section break, force a clean start from 1
        list.start = 1;
        justReset = false;
      } else if (nextStart > 0) {
        // Continue numbering across soft breaks
        list.start = nextStart;
      }
      nextStart = (list.start || 1) + list.children.length;
    } else if (SECTION_BREAK_TYPES.has(node.type)) {
      nextStart = 0; // structural break — reset counter
      justReset = true;
    } else {
      justReset = false; // any other node consumes the reset flag
    }
    // paragraph / ul / blockquote / code / etc.: preserve nextStart

    // Recurse into containers for nested ordered lists
    if ("children" in node && Array.isArray((node as { children: unknown[] }).children)) {
      fixOrderedLists((node as { children: Root["children"] }).children);
    }
  }
}

export default remarkContinuousOrderedList;
