import { useEffect, useRef, useCallback } from "react";
import { Crepe, CrepeFeature } from "@milkdown/crepe";
import { replaceAll } from "@milkdown/kit/utils";
import { languages } from "@codemirror/language-data";
import { json } from "@codemirror/lang-json";
import "@milkdown/crepe/theme/classic.css";
import "./milkdownTheme.css";

interface MilkdownPreviewProps {
  content: string;
  fontSize?: number;
  readonly?: boolean;
  onChange?: (markdown: string) => void;
}

export function MilkdownPreview({
  content,
  fontSize = 14,
  readonly = false,
  onChange,
}: MilkdownPreviewProps) {
  const containerRef = useRef<HTMLDivElement>(null);
  const crepeRef = useRef<Crepe | null>(null);
  const readyRef = useRef(false);
  const suppressRef = useRef(false);
  const onChangeRef = useRef(onChange);
  onChangeRef.current = onChange;

  const createEditor = useCallback(async (el: HTMLElement, value: string) => {
    const crepe = new Crepe({
      root: el,
      defaultValue: value,
      features: {
        [CrepeFeature.Toolbar]: false,
        [CrepeFeature.BlockEdit]: false,
        [CrepeFeature.LinkTooltip]: false,
        [CrepeFeature.CodeMirror]: true,
        [CrepeFeature.ListItem]: true,
        [CrepeFeature.Table]: true,
        [CrepeFeature.Latex]: true,
        [CrepeFeature.ImageBlock]: false,
        [CrepeFeature.Cursor]: false,
        [CrepeFeature.Placeholder]: false,
      },
      featureConfigs: {
        [CrepeFeature.CodeMirror]: {
          languages,
          extensions: [json()],
        },
      },
    });

    crepe.on((listener) => {
      listener.markdownUpdated((_ctx, markdown, prevMarkdown) => {
        if (markdown === prevMarkdown) return;
        if (suppressRef.current) return;
        onChangeRef.current?.(markdown);
      });
    });

    await crepe.create();
    return crepe;
  }, []);

  useEffect(() => {
    const el = containerRef.current;
    if (!el) return;

    let destroyed = false;

    createEditor(el, content).then((crepe) => {
      if (destroyed) {
        crepe.destroy();
        return;
      }
      crepe.setReadonly(readonly);
      crepeRef.current = crepe;
      readyRef.current = true;
    });

    return () => {
      destroyed = true;
      readyRef.current = false;
      if (crepeRef.current) {
        crepeRef.current.destroy();
        crepeRef.current = null;
      }
    };
  // eslint-disable-next-line -- only run on mount
  }, []);

  useEffect(() => {
    if (!readyRef.current || !crepeRef.current) return;
    crepeRef.current.setReadonly(readonly);
  }, [readonly]);

  useEffect(() => {
    if (!readyRef.current || !crepeRef.current) return;
    const current = crepeRef.current.getMarkdown();
    if (current === content) return;
    try {
      suppressRef.current = true;
      crepeRef.current.editor.action(replaceAll(content));
    } catch {
      // editor may not be fully ready yet
    } finally {
      suppressRef.current = false;
    }
  }, [content]);

  return (
    <div
      ref={containerRef}
      className="milkdown-preview font-body"
      style={{ fontSize: `${fontSize}px` }}
    />
  );
}
