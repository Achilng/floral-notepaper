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

  test("renders file links with their href preserved for navigation", () => {
    // react-markdown 会把 href 中的非 ASCII 字符百分号编码，
    // 预览点击侧再由 classifyMarkdownLink 解码
    const markup = renderToStaticMarkup(
      <MarkdownPreview content="[周报](./周报.md#概述)\n\n[官网](https://example.com)" />,
    );

    expect(markup).toContain('href="./%E5%91%A8%E6%8A%A5.md#%E6%A6%82%E8%BF%B0"');
    expect(markup).toContain('href="https://example.com"');
  });

  test("keeps file url hrefs that react-markdown would blank out by default", () => {
    const content = "[笔记](file:///D:/docs/笔记.md)";
    for (const renderHtml of [false, true]) {
      const markup = renderToStaticMarkup(
        <MarkdownPreview content={content} renderHtml={renderHtml} />,
      );
      expect(markup).toContain('href="file:///D:/docs/%E7%AC%94%E8%AE%B0.md"');
    }
  });
});
