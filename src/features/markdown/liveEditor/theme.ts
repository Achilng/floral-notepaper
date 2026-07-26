import { EditorView } from "@codemirror/view";
import { HighlightStyle, syntaxHighlighting } from "@codemirror/language";
import { tags as t } from "@lezer/highlight";

/**
 * CodeMirror theme for the WYSIWYG live editor. Colors are pulled from the
 * app's CSS custom properties so light/dark themes follow automatically.
 */
export function liveEditorTheme(fontSize: number) {
  return EditorView.theme({
    "&": {
      color: "var(--color-ink-soft)",
      backgroundColor: "transparent",
      fontSize: `${fontSize}px`,
      height: "100%",
    },
    ".cm-scroller": {
      fontFamily: "var(--font-body)",
      lineHeight: "1.9",
      overflow: "auto",
      padding: "0",
    },
    ".cm-content": {
      caretColor: "var(--color-bamboo)",
      padding: "4px 0 40vh 0",
      maxWidth: "100%",
    },
    "&.cm-focused": {
      outline: "none",
    },
    ".cm-cursor, .cm-dropCursor": {
      borderLeftColor: "var(--color-bamboo)",
      borderLeftWidth: "2px",
    },
    "&.cm-focused .cm-selectionBackground, .cm-selectionBackground, ::selection": {
      backgroundColor: "color-mix(in srgb, var(--color-bamboo) 22%, transparent)",
    },
    ".cm-line": {
      padding: "0 2px",
    },
    ".cm-gutters": {
      backgroundColor: "transparent",
      color: "var(--color-ink-ghost)",
      border: "none",
      borderRight: "1px solid color-mix(in srgb, var(--color-paper-deep) 70%, transparent)",
      marginRight: "0.6em",
    },
    ".cm-lineNumbers .cm-gutterElement": {
      color: "var(--color-ink-ghost)",
      opacity: "0.55",
      minWidth: "2.2em",
      padding: "0 0.5em 0 0.6em",
      fontVariantNumeric: "tabular-nums",
    },
    ".cm-activeLine, .cm-md-active-block": {
      backgroundColor: "color-mix(in srgb, var(--color-bamboo) 8%, transparent)",
    },
    // “块+行”模式下块内当前行：比块底色(8%)略强但仍克制，避免遮蔽文字。
    // 定义晚于 .cm-md-active-block，同优先级时此规则胜出。
    ".cm-md-active-block-line": {
      backgroundColor: "color-mix(in srgb, var(--color-bamboo) 15%, transparent)",
    },

    // --- Markdown live-preview element styling ---
    ".cm-md-h1": {
      fontSize: "1.57em",
      fontWeight: "700",
      fontFamily: "var(--font-display)",
      color: "var(--color-ink)",
      lineHeight: "1.5",
    },
    ".cm-md-h2": {
      fontSize: "1.32em",
      fontWeight: "700",
      fontFamily: "var(--font-display)",
      color: "var(--color-ink)",
      lineHeight: "1.5",
    },
    ".cm-md-h3": {
      fontSize: "1.15em",
      fontWeight: "700",
      fontFamily: "var(--font-display)",
      color: "var(--color-ink)",
      lineHeight: "1.6",
    },
    ".cm-md-h4, .cm-md-h5, .cm-md-h6": {
      fontSize: "1.04em",
      fontWeight: "600",
      fontFamily: "var(--font-display)",
      color: "var(--color-ink)",
      lineHeight: "1.6",
    },
    ".cm-md-strong": {
      fontWeight: "700",
      color: "var(--color-ink)",
    },
    ".cm-md-em": {
      fontStyle: "italic",
      color: "var(--color-bamboo-light)",
    },
    ".cm-md-strike": {
      textDecoration: "line-through",
      color: "var(--color-ink-faint)",
    },
    ".cm-md-inline-code": {
      fontFamily: "var(--font-mono)",
      fontSize: "0.88em",
      backgroundColor: "var(--color-paper-warm)",
      color: "var(--color-bamboo)",
      borderRadius: "4px",
      padding: "0.1em 0.35em",
    },
    ".cm-md-link": {
      color: "var(--color-bamboo)",
      textDecoration: "underline",
      textUnderlineOffset: "2px",
      cursor: "pointer",
    },
    ".cm-md-url, .cm-md-mark-hidden": {
      color: "var(--color-ink-ghost)",
    },
    ".cm-md-quote": {
      color: "color-mix(in srgb, var(--color-ink-soft) 80%, transparent)",
      fontStyle: "italic",
      borderLeft: "2px solid color-mix(in srgb, var(--color-bamboo) 40%, transparent)",
      paddingLeft: "0.9em !important",
    },
    ".cm-md-list-line": {
      // nothing special yet, hook for future
    },

    // --- Fenced code block ---
    ".cm-md-code-block": {
      backgroundColor: "color-mix(in srgb, var(--color-paper-warm) 80%, transparent)",
      fontFamily: "var(--font-mono)",
      fontSize: "0.88em",
      paddingLeft: "1em",
      paddingRight: "1em",
    },
    ".cm-md-code-block-first": {
      borderTopLeftRadius: "6px",
      borderTopRightRadius: "6px",
      paddingTop: "0.3em !important",
    },
    ".cm-md-code-block-last": {
      borderBottomLeftRadius: "6px",
      borderBottomRightRadius: "6px",
      paddingBottom: "0.3em !important",
    },
    ".cm-md-fence-mark": {
      color: "var(--color-ink-ghost)",
      fontFamily: "var(--font-mono)",
      fontSize: "0.85em",
    },

    // --- Widgets ---
    ".cm-md-hr": {
      display: "inline-block",
      width: "100%",
      verticalAlign: "middle",
      height: "1px",
      backgroundColor: "var(--color-paper-deep)",
    },
    ".cm-md-task": {
      marginRight: "0.4em",
      accentColor: "var(--color-bamboo)",
      cursor: "pointer",
      verticalAlign: "middle",
    },
    ".cm-md-image": {
      display: "block",
      maxWidth: "60%",
      margin: "0.4em auto",
      borderRadius: "4px",
    },
    ".cm-md-bullet, .cm-md-bullet-raw": {
      display: "inline-block",
      width: "1.3em",
      textAlign: "center",
    },
    ".cm-md-bullet": {
      color: "var(--color-bamboo)",
    },
    ".cm-md-bullet-raw": {
      color: "var(--color-ink-ghost)",
    },
  });
}

/**
 * Syntax highlighting for code inside fenced blocks. Kept muted to match the
 * paper aesthetic.
 */
export const liveHighlightStyle = HighlightStyle.define([
  { tag: t.keyword, color: "var(--color-bamboo)" },
  { tag: [t.string, t.special(t.string)], color: "#a06a3d" },
  {
    tag: [t.comment, t.lineComment, t.blockComment],
    color: "var(--color-ink-ghost)",
    fontStyle: "italic",
  },
  { tag: [t.number, t.bool, t.null], color: "#9a5b9a" },
  { tag: [t.function(t.variableName), t.function(t.propertyName)], color: "#3a6ea5" },
  { tag: [t.typeName, t.className, t.namespace], color: "#2d7d6f" },
  { tag: [t.propertyName, t.attributeName], color: "#7a6a3d" },
  { tag: [t.operator, t.punctuation], color: "var(--color-ink-faint)" },
  { tag: [t.variableName], color: "var(--color-ink-soft)" },
  { tag: t.invalid, color: "#d04545" },
]);

export const liveHighlighting = syntaxHighlighting(liveHighlightStyle);
