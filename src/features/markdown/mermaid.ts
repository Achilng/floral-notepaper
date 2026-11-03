// Initialize and render together: Mermaid's configuration is shared between diagrams.
let renderQueue: Promise<unknown> = Promise.resolve();
let nextId = 0;

export function renderMermaid(source: string, dark: boolean): Promise<string> {
  const render = renderQueue.then(async () => {
    const { default: mermaid } = await import("mermaid");
    mermaid.initialize({
      startOnLoad: false,
      securityLevel: "strict",
      suppressErrorRendering: true,
      theme: dark ? "dark" : "default",
      look: "classic",
      layout: "elk",
      elk: {
        lineHops: true,
      },
      flowchart: { htmlLabels: false },
      secure: [
        "secure",
        "securityLevel",
        "startOnLoad",
        "maxTextSize",
        "suppressErrorRendering",
        "maxEdges",
      ],
    });

    // Keep Mermaid's measurement nodes out of the preview, including on syntax errors.
    const container = document.createElement("div");
    container.style.cssText = "position:fixed;left:-100000px;top:0;visibility:hidden";
    document.body.append(container);
    try {
      const { svg } = await mermaid.render(`mermaid-diagram-${nextId++}`, source, container);
      return svg;
    } finally {
      container.remove();
    }
  });
  // A malformed diagram must not block subsequent renders.
  renderQueue = render.catch(() => {});
  return render;
}
