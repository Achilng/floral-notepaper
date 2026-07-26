import { useEffect, useRef } from "react";
import { Annotation, Compartment, EditorState, type TransactionSpec } from "@codemirror/state";
import {
  EditorView,
  highlightActiveLine,
  keymap,
  lineNumbers,
  placeholder as cmPlaceholder,
} from "@codemirror/view";
import { history, historyKeymap, defaultKeymap, indentWithTab } from "@codemirror/commands";
import { markdown, markdownLanguage, markdownKeymap } from "@codemirror/lang-markdown";
import { codeFolding } from "@codemirror/language";
import { languages } from "@codemirror/language-data";

const codeFoldingExtension = codeFolding({
  placeholderDOM(_view, onclick) {
    const el = document.createElement("span");
    el.className = "cm-md-fold-placeholder";
    el.textContent = "⋯";
    el.title = "展开";
    el.setAttribute("aria-label", "展开代码块");
    el.onclick = onclick;
    return el;
  },
});
import { liveEditorTheme, liveHighlighting } from "./liveEditor/theme";
import { livePreview } from "./liveEditor/livePreview";

export interface LiveEditorProps {
  value: string;
  onChange: (value: string) => void;
  fontSize?: number;
  placeholder?: string;
  readOnly?: boolean;
  autoFocus?: boolean;
  resolveImageSrc?: (src: string) => string;
  showCodeLineNumbers?: boolean;
  showEditorLineNumbers?: boolean;
  activeHighlight?: "off" | "line" | "block" | "block-line";
  /** When false, fenced code blocks scroll horizontally instead of wrapping. */
  codeWrap?: boolean;
  /**
   * Identity of the document currently shown (e.g. note id / external file path).
   * Lets value-sync tell a *switch* (docKey changes → cursor may reset) apart from
   * an in-place *reload* of the same doc (docKey unchanged → preserve the cursor).
   */
  docKey?: string | number;
}

const identity = (src: string) => src;

// 标记"把外部 value 同步进编辑器"的事务（切换笔记/外部文件时触发）。
// 这类事务不是用户编辑，updateListener 据此跳过 onChange，避免新载入的笔记被误标脏
// → 否则下一次切换会触发"保存"（外部文件会被无谓回写到磁盘、改 mtime）。
const externalSync = Annotation.define<boolean>();

export function LiveEditor({
  value,
  onChange,
  fontSize = 14,
  placeholder,
  readOnly = false,
  autoFocus = false,
  resolveImageSrc = identity,
  showCodeLineNumbers = false,
  showEditorLineNumbers = false,
  activeHighlight = "off",
  codeWrap = true,
  docKey,
}: LiveEditorProps) {
  const hostRef = useRef<HTMLDivElement>(null);
  const viewRef = useRef<EditorView | null>(null);
  // Tracks the docKey of the value currently in the editor (see value-sync below).
  const lastDocKeyRef = useRef(docKey);
  const themeCompartment = useRef(new Compartment());
  const previewCompartment = useRef(new Compartment());
  const editableCompartment = useRef(new Compartment());
  const lineNumbersCompartment = useRef(new Compartment());
  const activeLineCompartment = useRef(new Compartment());

  // Keep latest callbacks/values accessible from CodeMirror without rebuilding.
  const onChangeRef = useRef(onChange);
  onChangeRef.current = onChange;
  const resolveImageSrcRef = useRef(resolveImageSrc);
  resolveImageSrcRef.current = resolveImageSrc;
  const showCodeLineNumbersRef = useRef(showCodeLineNumbers);
  showCodeLineNumbersRef.current = showCodeLineNumbers;
  const activeHighlightRef = useRef(activeHighlight);
  activeHighlightRef.current = activeHighlight;

  const makePreviewExtension = () =>
    livePreview({
      resolveImageSrc: (src) => resolveImageSrcRef.current(src),
      showCodeLineNumbers: showCodeLineNumbersRef.current,
      activeBlock:
        activeHighlightRef.current === "block" || activeHighlightRef.current === "block-line",
      activeLineInBlock: activeHighlightRef.current === "block-line",
    });

  // Create the editor once on mount.
  useEffect(() => {
    if (!hostRef.current) return;

    const view = new EditorView({
      parent: hostRef.current,
      state: EditorState.create({
        doc: value,
        extensions: [
          keymap.of([...markdownKeymap, indentWithTab, ...defaultKeymap, ...historyKeymap]),
          history(),
          lineNumbersCompartment.current.of(showEditorLineNumbers ? lineNumbers() : []),
          activeLineCompartment.current.of(activeHighlight === "line" ? highlightActiveLine() : []),
          EditorView.lineWrapping,
          markdown({ base: markdownLanguage, codeLanguages: languages }),
          codeFoldingExtension,
          liveHighlighting,
          themeCompartment.current.of(liveEditorTheme(fontSize)),
          previewCompartment.current.of(makePreviewExtension()),
          editableCompartment.current.of(EditorView.editable.of(!readOnly)),
          cmPlaceholder(placeholder ?? ""),
          EditorView.updateListener.of((update) => {
            if (update.docChanged) {
              // 外部 value 同步（切换笔记/文件）不算用户编辑，不上报 onChange（不标脏）；
              // 真实键入、撤销/重做等不带此注解，照常上报。
              const isExternalSync = update.transactions.some((tr) => tr.annotation(externalSync));
              if (!isExternalSync) {
                onChangeRef.current(update.state.doc.toString());
              }
            }
          }),
        ],
      }),
    });
    viewRef.current = view;

    if (autoFocus) view.focus();

    return () => {
      view.destroy();
      viewRef.current = null;
    };
    // Editor is created once; prop changes are handled by the effects below.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  // Sync external value changes (switching notes / external-file reloads) into the editor.
  useEffect(() => {
    const view = viewRef.current;
    if (!view) return;
    // 是否仍是同一篇文档（仅 value 变、docKey 未变）= 原地回读，需保留光标。
    const sameDoc = docKey !== undefined && docKey === lastDocKeyRef.current;
    lastDocKeyRef.current = docKey;

    const current = view.state.doc.toString();
    if (value === current) return;

    const spec: TransactionSpec = {
      changes: { from: 0, to: current.length, insert: value },
      annotations: externalSync.of(true),
    };
    // 原地回读（如外部文件 mtime 轮询、被别的程序改动后重载）整篇替换会丢光标 →
    // 保留并钳制原选区，避免光标乱跳。切换到另一篇（docKey 改变）则不保留、沿用归位行为。
    if (sameDoc) {
      const prev = view.state.selection.main;
      spec.selection = {
        anchor: Math.min(prev.anchor, value.length),
        head: Math.min(prev.head, value.length),
      };
    }
    view.dispatch(spec);
  }, [value, docKey]);

  // React to font-size changes.
  useEffect(() => {
    const view = viewRef.current;
    if (!view) return;
    view.dispatch({
      effects: themeCompartment.current.reconfigure(liveEditorTheme(fontSize)),
    });
  }, [fontSize]);

  // React to image-resolver or code-line-number changes (rebuild decorations).
  useEffect(() => {
    const view = viewRef.current;
    if (!view) return;
    view.dispatch({
      effects: previewCompartment.current.reconfigure(makePreviewExtension()),
    });
    // makePreviewExtension reads the latest values via refs.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [resolveImageSrc, showCodeLineNumbers, activeHighlight]);

  // React to read-only changes.
  useEffect(() => {
    const view = viewRef.current;
    if (!view) return;
    view.dispatch({
      effects: editableCompartment.current.reconfigure(EditorView.editable.of(!readOnly)),
    });
  }, [readOnly]);

  // Toggle the whole-document line-number gutter.
  useEffect(() => {
    const view = viewRef.current;
    if (!view) return;
    view.dispatch({
      effects: lineNumbersCompartment.current.reconfigure(
        showEditorLineNumbers ? lineNumbers() : [],
      ),
    });
  }, [showEditorLineNumbers]);

  // Toggle active-line highlighting (block highlighting is handled in livePreview).
  useEffect(() => {
    const view = viewRef.current;
    if (!view) return;
    view.dispatch({
      effects: activeLineCompartment.current.reconfigure(
        activeHighlight === "line" ? highlightActiveLine() : [],
      ),
    });
  }, [activeHighlight]);

  return (
    <div
      ref={hostRef}
      className={`cm-live-editor h-full overflow-hidden ${codeWrap ? "" : "code-nowrap"}`}
    />
  );
}
