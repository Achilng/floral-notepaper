import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, test, vi } from "vitest";
import { MarkdownPreview } from "./MarkdownPreview";

vi.mock("@tauri-apps/plugin-opener", () => ({
  openUrl: vi.fn(),
}));

describe("MarkdownPreview", () => {
  test("marks rendered Markdown content as selectable", () => {
    const markup = renderToStaticMarkup(<MarkdownPreview content="# 花笺\n\n正文" />);

    expect(markup).toContain("markdown-selectable");
    expect(markup).toContain("<h1");
    expect(markup).toContain("花笺");
    expect(markup).toContain("正文");
  });

  test("renders ==highlight== as a themed mark element", () => {
    const markup = renderToStaticMarkup(<MarkdownPreview content="前 ==重点== 后" />);

    expect(markup).toContain('<mark class="markdown-highlight">重点</mark>');
  });

  test("renders ==highlight== alongside raw <mark> in the HTML pipeline", () => {
    const markup = renderToStaticMarkup(
      <MarkdownPreview content={"==重点== 与 <mark>原生</mark>"} renderHtml />,
    );

    expect(markup).toContain('<mark class="markdown-highlight">重点</mark>');
    expect(markup).toContain("<mark>原生</mark>");
  });

  test("keeps ==highlight== markers literal inside code blocks", () => {
    const markup = renderToStaticMarkup(<MarkdownPreview content={"```text\n==x==\n```"} />);

    expect(markup).not.toContain("markdown-highlight");
    expect(markup).toContain("==x==");
  });

  test("keeps code block controls outside the horizontally scrollable pre", () => {
    const markup = renderToStaticMarkup(
      <MarkdownPreview content={"```text\nvery long code line\n```"} />,
    );

    const preCloseIndex = markup.indexOf("</pre>");
    const buttonIndex = markup.indexOf("<button");

    expect(markup).toContain("markdown-code-block");
    expect(markup).toContain("markdown-code-scroll");
    expect(preCloseIndex).toBeGreaterThan(-1);
    expect(buttonIndex).toBeGreaterThan(preCloseIndex);
  });
});
