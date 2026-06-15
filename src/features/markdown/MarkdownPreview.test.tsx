import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, test, vi } from "vitest";
import { MarkdownPreview } from "./MarkdownPreview";

vi.mock("@tauri-apps/plugin-opener", () => ({
  openUrl: vi.fn(),
}));

vi.mock("./Mermaid", () => ({
  Mermaid: ({ chart }: { chart: string }) => <div data-testid="mermaid">{chart}</div>,
}));

describe("MarkdownPreview", () => {
  test("marks rendered Markdown content as selectable", () => {
    const markup = renderToStaticMarkup(<MarkdownPreview content="# 花笺\n\n正文" />);

    expect(markup).toContain("markdown-selectable");
    expect(markup).toContain("<h1");
    expect(markup).toContain("花笺");
    expect(markup).toContain("正文");
  });

  test("renders Mermaid charts using the Mermaid component", () => {
    const markup = renderToStaticMarkup(
      <MarkdownPreview content={"```mermaid\ngraph TD\nA --> B\n```"} />,
    );

    expect(markup).toContain('data-testid="mermaid"');
    expect(markup).toContain("graph TD");
    expect(markup).toContain("A --&gt; B");
  });
});
