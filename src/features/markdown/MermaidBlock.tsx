import { useEffect, useState } from "react";
import type { ReactNode } from "react";
import { useTranslation } from "react-i18next";
import { renderMermaid } from "./mermaid";

export function MermaidBlock({ source, children }: { source: string; children: ReactNode }) {
  const { t } = useTranslation();
  const [dark, setDark] = useState(() =>
    typeof document === "undefined" ? false : document.documentElement.dataset.theme === "dark",
  );
  const [result, setResult] = useState<{
    source: string;
    dark: boolean;
    svg?: string;
    failed?: boolean;
  }>();

  useEffect(() => {
    const root = document.documentElement;
    const update = () => setDark(root.dataset.theme === "dark");
    update();
    const observer = new MutationObserver(update);
    observer.observe(root, { attributes: true, attributeFilter: ["data-theme"] });
    return () => observer.disconnect();
  }, []);

  useEffect(() => {
    let cancelled = false;
    void renderMermaid(source, dark).then(
      (svg) => {
        if (!cancelled) setResult({ source, dark, svg });
      },
      () => {
        if (!cancelled) setResult({ source, dark, failed: true });
      },
    );
    return () => {
      cancelled = true;
    };
  }, [source, dark]);

  const current = result?.source === source && result.dark === dark ? result : undefined;
  return (
    <div className="markdown-mermaid rounded bg-paper-warm/80 px-4 pt-8 pb-3 overflow-x-auto">
      {current?.svg ? (
        <div
          className="markdown-mermaid-diagram"
          role="img"
          aria-label={t("markdown.mermaidDiagram")}
          // Mermaid sanitizes the generated SVG in strict security mode.
          dangerouslySetInnerHTML={{ __html: current.svg }}
        />
      ) : (
        <>
          <p role="status" className="text-ink-ghost text-[0.85em]">
            {t(current?.failed ? "markdown.mermaidError" : "markdown.mermaidLoading")}
          </p>
          <pre className="markdown-code-scroll m-0 overflow-x-auto">{children}</pre>
        </>
      )}
    </div>
  );
}
