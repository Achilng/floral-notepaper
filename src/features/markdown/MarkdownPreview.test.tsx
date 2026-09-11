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

  test("does not strike through a lone tilde between numbers", () => {
    const markup = renderToStaticMarkup(<MarkdownPreview content="0.2~3 μm / 微藻 3~20 μm" />);

    expect(markup).not.toContain("<del");
    expect(markup).toContain("0.2~3 μm");
    expect(markup).toContain("3~20 μm");
  });

  test("still renders double-tilde strikethrough", () => {
    const markup = renderToStaticMarkup(<MarkdownPreview content="~~已完成~~" />);

    expect(markup).toContain("<del");
    expect(markup).toContain("已完成");
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
