import { useEffect, useRef, useState } from "react";
import mermaid from "mermaid";

// Initialize mermaid
mermaid.initialize({
  startOnLoad: false,
  theme: "default",
  securityLevel: "loose",
});

interface MermaidProps {
  chart: string;
}

export function Mermaid({ chart }: MermaidProps) {
  const [svg, setSvg] = useState<string>("");
  const [error, setError] = useState<string | null>(null);
  const containerId = useRef(`mermaid-${Math.random().toString(36).substring(2, 9)}`);

  useEffect(() => {
    let active = true;
    const renderChart = async () => {
      try {
        const { svg: renderedSvg } = await mermaid.render(containerId.current, chart.trim());
        if (active) {
          setSvg(renderedSvg);
          setError(null);
        }
      } catch (err) {
        if (active) {
          console.error("Mermaid render error:", err);
          setError("Failed to render Mermaid chart");
        }
      }
    };

    void renderChart();

    return () => {
      active = false;
    };
  }, [chart]);

  if (error) {
    return (
      <div className="my-3 p-3 text-xs font-mono bg-red-100/10 text-red-500 rounded border border-red-500/20">
        <div>⚠️ {error}</div>
        <pre className="mt-2 text-[10px] overflow-x-auto whitespace-pre-wrap">{chart}</pre>
      </div>
    );
  }

  if (!svg) {
    return (
      <div className="my-3 p-4 bg-paper-warm/50 text-ink-ghost/60 rounded flex items-center justify-center font-mono text-xs animate-pulse">
        Rendering chart...
      </div>
    );
  }

  return (
    <div
      className="my-4 flex justify-center bg-paper-warm/20 rounded p-2 overflow-x-auto"
      dangerouslySetInnerHTML={{ __html: svg }}
    />
  );
}
