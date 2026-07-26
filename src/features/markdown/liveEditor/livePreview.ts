import { StateField, type EditorState, type Extension, type Range } from "@codemirror/state";
import { Decoration, type DecorationSet, EditorView } from "@codemirror/view";
import { foldEffect, foldedRanges, syntaxTree, unfoldEffect } from "@codemirror/language";
import type { SyntaxNode } from "@lezer/common";
import {
  BulletWidget,
  CheckboxWidget,
  CodeToolbarWidget,
  HorizontalRuleWidget,
  ImageWidget,
  TableWidget,
} from "./widgets";

export interface LivePreviewOptions {
  /** Maps a markdown image src to a URL usable in an <img> tag. */
  resolveImageSrc: (src: string) => string;
  /** Show 1-based line numbers inside fenced code blocks. */
  showCodeLineNumbers?: boolean;
  /** Highlight the whole markdown block the cursor sits in. */
  activeBlock?: boolean;
  /** Within the active block, give the cursor's own line a stronger tint ("block-line" mode). */
  activeLineInBlock?: boolean;
}

const LIST_LEVEL_INDENT_EM = 1.5;
const activeBlockLine = Decoration.line({ class: "cm-md-active-block" });
// 块内当前行的更强高光（“块+行”模式）。CSS 定义晚于 active-block，同类同优先级时胜出。
const activeBlockCurrentLine = Decoration.line({ class: "cm-md-active-block-line" });
// Block-level nodes whose extent defines "the current block" for highlighting.
const BLOCK_NAMES = new Set([
  "Paragraph",
  "ATXHeading1",
  "ATXHeading2",
  "ATXHeading3",
  "ATXHeading4",
  "ATXHeading5",
  "ATXHeading6",
  "SetextHeading1",
  "SetextHeading2",
  "FencedCode",
  "Blockquote",
  "ListItem",
  "Table",
  "HorizontalRule",
]);
const hideMark = Decoration.replace({});
const strong = Decoration.mark({ class: "cm-md-strong" });
const em = Decoration.mark({ class: "cm-md-em" });
const strike = Decoration.mark({ class: "cm-md-strike" });
const inlineCode = Decoration.mark({ class: "cm-md-inline-code" });
const linkText = Decoration.mark({ class: "cm-md-link" });
const fenceMark = Decoration.mark({ class: "cm-md-fence-mark" });
const bulletRaw = Decoration.mark({ class: "cm-md-bullet-raw" });
const quoteLine = Decoration.line({ class: "cm-md-quote" });
const tableSourceLine = Decoration.line({ class: "cm-md-table-source" });
const headingLine = [
  null,
  Decoration.line({ class: "cm-md-h1" }),
  Decoration.line({ class: "cm-md-h2" }),
  Decoration.line({ class: "cm-md-h3" }),
  Decoration.line({ class: "cm-md-h4" }),
  Decoration.line({ class: "cm-md-h5" }),
  Decoration.line({ class: "cm-md-h6" }),
];

function directChildren(node: SyntaxNode): Array<{ name: string; from: number; to: number }> {
  const out: Array<{ name: string; from: number; to: number }> = [];
  const cursor = node.cursor();
  if (cursor.firstChild()) {
    do {
      out.push({ name: cursor.name, from: cursor.from, to: cursor.to });
    } while (cursor.nextSibling());
  }
  return out;
}

function buildDecorations(state: EditorState, options: LivePreviewOptions): DecorationSet {
  const { doc } = state;
  if (doc.length === 0) return Decoration.none;
  const decorations: Array<Range<Decoration>> = [];

  // A construct is "active" (show raw source) when any selection range sits on
  // one of the lines it spans — the Typora / Obsidian active-line behaviour.
  const isActive = (from: number, to: number): boolean => {
    const startLine = doc.lineAt(from).number;
    const endLine = doc.lineAt(to).number;
    for (const range of state.selection.ranges) {
      const a = doc.lineAt(range.from).number;
      const b = doc.lineAt(range.to).number;
      if (a <= endLine && b >= startLine) return true;
    }
    return false;
  };

  const hideWithTrailingSpace = (from: number, to: number) => {
    let end = to;
    if (doc.sliceString(end, end + 1) === " ") end += 1;
    decorations.push(hideMark.range(from, end));
  };

  // Highlight every line of the block the cursor currently sits in. In "block-line"
  // mode also give the cursor's own line a stronger tint inside that block.
  if (options.activeBlock) {
    const pos = state.selection.main.head;
    const cursorLine = doc.lineAt(pos).number;
    let node: SyntaxNode | null = syntaxTree(state).resolveInner(pos, 0);
    while (node && !BLOCK_NAMES.has(node.name)) node = node.parent;
    // 空行 / 段落间隙没有块级祖先（resolveInner 落到 Document）→ 退而高亮光标当前行，
    // 保证当前行始终被“覆盖”（修“单行不显示高光”）。
    const startLine = node ? doc.lineAt(node.from).number : cursorLine;
    const endLine = node ? doc.lineAt(Math.min(node.to, doc.length)).number : cursorLine;
    for (let l = startLine; l <= endLine; l++) {
      decorations.push(activeBlockLine.range(doc.line(l).from));
      if (options.activeLineInBlock && l === cursorLine) {
        decorations.push(activeBlockCurrentLine.range(doc.line(l).from));
      }
    }
  }

  try {
    syntaxTree(state).iterate({
      enter: (nodeRef) => {
        const { name, from, to } = nodeRef;

        const headingMatch = /^ATXHeading(\d)$/.exec(name);
        if (headingMatch) {
          const level = Number(headingMatch[1]);
          decorations.push(headingLine[level]!.range(doc.lineAt(from).from));
          if (!isActive(from, to)) {
            for (const mark of directChildren(nodeRef.node)) {
              if (mark.name === "HeaderMark") hideWithTrailingSpace(mark.from, mark.to);
            }
          }
          return undefined;
        }

        if (name === "StrongEmphasis" || name === "Emphasis" || name === "Strikethrough") {
          const style = name === "StrongEmphasis" ? strong : name === "Emphasis" ? em : strike;
          decorations.push(style.range(from, to));
          if (!isActive(from, to)) {
            for (const mark of directChildren(nodeRef.node)) {
              if (mark.name === "EmphasisMark" || mark.name === "StrikethroughMark") {
                decorations.push(hideMark.range(mark.from, mark.to));
              }
            }
          }
          return undefined;
        }

        if (name === "InlineCode") {
          decorations.push(inlineCode.range(from, to));
          if (!isActive(from, to)) {
            for (const mark of directChildren(nodeRef.node)) {
              if (mark.name === "CodeMark") decorations.push(hideMark.range(mark.from, mark.to));
            }
          }
          return false;
        }

        if (name === "Link") {
          if (!isActive(from, to)) {
            const marks = directChildren(nodeRef.node).filter((k) => k.name === "LinkMark");
            if (marks.length >= 2) {
              const open = marks[0];
              const close = marks[1];
              if (close.from > open.to) decorations.push(linkText.range(open.to, close.from));
              decorations.push(hideMark.range(open.from, open.to));
              decorations.push(hideMark.range(close.from, to));
            }
          }
          return undefined;
        }

        if (name === "Image") {
          if (!isActive(from, to)) {
            const kids = directChildren(nodeRef.node);
            const urlNode = kids.find((k) => k.name === "URL");
            const marks = kids.filter((k) => k.name === "LinkMark");
            const alt = marks.length >= 2 ? doc.sliceString(marks[0].to, marks[1].from) : "";
            const src = urlNode ? doc.sliceString(urlNode.from, urlNode.to) : "";
            if (src) {
              decorations.push(
                Decoration.replace({
                  widget: new ImageWidget(src, alt, options.resolveImageSrc),
                }).range(from, to),
              );
              return false;
            }
          }
          return undefined;
        }

        if (name === "HorizontalRule") {
          if (!isActive(from, to)) {
            decorations.push(
              Decoration.replace({ widget: new HorizontalRuleWidget() }).range(from, to),
            );
          }
          return false;
        }

        if (name === "FencedCode") {
          const startLine = doc.lineAt(from).number;
          const endLine = doc.lineAt(to).number;
          for (let l = startLine; l <= endLine; l++) {
            const line = doc.line(l);
            const classes = ["cm-md-code-block"];
            if (l === startLine) classes.push("cm-md-code-block-first");
            if (l === endLine) classes.push("cm-md-code-block-last");
            decorations.push(Decoration.line({ class: classes.join(" ") }).range(line.from));
            // Number only the code content lines, not the ``` fence lines.
            if (options.showCodeLineNumbers && l > startLine && l < endLine) {
              decorations.push(
                Decoration.line({
                  class: "cm-md-code-content",
                  attributes: { "data-ln": String(l - startLine) },
                }).range(line.from),
              );
            }
          }
          // Top-right toolbar (language · copy · fold) anchored on the opening
          // fence line, outside the foldable range so it survives folding.
          const firstLine = doc.line(startLine);
          const kids = directChildren(nodeRef.node);
          const info = kids.find((k) => k.name === "CodeInfo");
          const text = kids.find((k) => k.name === "CodeText");
          const lang = info ? doc.sliceString(info.from, info.to).trim() : "";
          const foldTo = Math.min(to, doc.length);
          const codeFrom = text ? text.from : firstLine.to;
          const codeTo = text ? text.to : foldTo;
          let folded = false;
          foldedRanges(state).between(firstLine.to, foldTo, () => {
            folded = true;
          });
          decorations.push(
            Decoration.widget({
              widget: new CodeToolbarWidget(lang, firstLine.to, foldTo, codeFrom, codeTo, folded),
              side: -1,
            }).range(firstLine.from),
          );
          return undefined;
        }

        if (
          (name === "CodeMark" || name === "CodeInfo") &&
          nodeRef.node.parent?.name === "FencedCode"
        ) {
          decorations.push(fenceMark.range(from, to));
          return undefined;
        }

        if (name === "Blockquote") {
          const startLine = doc.lineAt(from).number;
          const endLine = doc.lineAt(to).number;
          for (let l = startLine; l <= endLine; l++) {
            decorations.push(quoteLine.range(doc.line(l).from));
          }
          return undefined;
        }

        if (name === "QuoteMark") {
          // Always collapse the ">" marker (no active-line reveal) so the quote
          // reads as one continuous block and the text never shifts horizontally.
          hideWithTrailingSpace(from, to);
          return undefined;
        }

        if (name === "ListItem") {
          // Normalize indentation: collapse the source's leading whitespace and
          // re-apply a consistent per-depth indent with a hanging indent so
          // wrapped lines align under the text instead of the marker.
          const line = doc.lineAt(from);
          let depth = 0;
          for (let p = nodeRef.node.parent; p; p = p.parent) {
            if (p.name === "BulletList" || p.name === "OrderedList") depth += 1;
          }
          const lineText = doc.sliceString(line.from, line.to);
          const leading = lineText.length - lineText.trimStart().length;
          if (leading > 0) decorations.push(hideMark.range(line.from, line.from + leading));
          const markerEm = 1.4;
          const padLeft = Math.max(0, depth - 1) * LIST_LEVEL_INDENT_EM + markerEm;
          decorations.push(
            Decoration.line({
              attributes: { style: `padding-left:${padLeft}em;text-indent:-${markerEm}em;` },
            }).range(line.from),
          );
          return undefined;
        }

        if (name === "ListMark") {
          // On the active line show the raw marker (-, *, +) for consistency with
          // other elements; otherwise render a round bullet. Both occupy the same
          // fixed width so toggling never shifts the text horizontally.
          if (/^[-*+]$/.test(doc.sliceString(from, to))) {
            if (isActive(from, to)) {
              decorations.push(bulletRaw.range(from, to));
            } else {
              decorations.push(Decoration.replace({ widget: new BulletWidget() }).range(from, to));
            }
          }
          return undefined;
        }

        if (name === "TaskMarker") {
          if (!isActive(from, to)) {
            const checked = /[xX]/.test(doc.sliceString(from, to));
            decorations.push(
              Decoration.replace({ widget: new CheckboxWidget(checked, from + 1) }).range(from, to),
            );
          }
          return false;
        }

        if (name === "Table") {
          const lineFrom = doc.lineAt(from).from;
          const lineTo = doc.lineAt(to).to;
          if (!isActive(lineFrom, lineTo)) {
            const source = doc.sliceString(lineFrom, lineTo);
            decorations.push(
              Decoration.replace({
                widget: new TableWidget(source, lineFrom),
                block: true,
              }).range(lineFrom, lineTo),
            );
            return false;
          }
          const startLine = doc.lineAt(from).number;
          const endLine = doc.lineAt(to).number;
          for (let l = startLine; l <= endLine; l++) {
            decorations.push(tableSourceLine.range(doc.line(l).from));
          }
          return undefined;
        }

        return undefined;
      },
    });
  } catch (error) {
    console.error("[liveEditor] failed to build decorations", error);
    return Decoration.none;
  }

  return Decoration.set(decorations, true);
}

export function livePreview(options: LivePreviewOptions): Extension {
  return StateField.define<DecorationSet>({
    create: (state) => buildDecorations(state, options),
    update: (value, tr) => {
      const foldChanged = tr.effects.some((e) => e.is(foldEffect) || e.is(unfoldEffect));
      if (tr.docChanged || tr.selection || foldChanged) {
        return buildDecorations(tr.state, options);
      }
      return value;
    },
    provide: (field) => EditorView.decorations.from(field),
  });
}
