import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, test, vi } from "vitest";
import { MarkdownPreview } from "./MarkdownPreview";
import { toggleTaskAtLine } from "./toggleTaskAtLine";

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

  test("renders single newlines as line breaks", () => {
    const markup = renderToStaticMarkup(<MarkdownPreview content={"first line\nsecond line"} />);

    expect(markup).toContain("<br");
  });

  test("renders single newlines inside lists and blockquotes as line breaks", () => {
    const markup = renderToStaticMarkup(
      <MarkdownPreview content={"- 列表第一行\n  列表第二行\n\n> 引用第一行\n> 引用第二行"} />,
    );

    expect(markup).toContain("<br");
  });

  test("preserves blank lines between paragraphs and inside blockquotes", () => {
    const markup = renderToStaticMarkup(
      <MarkdownPreview content={"first\n\n\nsecond\n\n> 引用第一行\n>\n> 引用第二行"} />,
    );

    expect(markup.match(/<br/g)).toHaveLength(4);
  });

  test("keeps task checkboxes read-only without a toggle callback", () => {
    const markup = renderToStaticMarkup(<MarkdownPreview content={"- [ ] todo\n- [x] done"} />);

    expect(markup).toContain('type="checkbox"');
    expect(markup).toContain("disabled");
  });

  test("enables task checkboxes with a toggle callback", () => {
    const markup = renderToStaticMarkup(
      <MarkdownPreview content={"- [ ] todo\n- [x] done"} onToggleTask={vi.fn()} />,
    );

    expect(markup).not.toContain("disabled");
  });

  test("toggles task state on the matching source line", () => {
    const source = "- [ ] todo\n- [x] done";

    expect(toggleTaskAtLine(source, 1)).toBe("- [x] todo\n- [x] done");
    expect(toggleTaskAtLine(source, 2)).toBe("- [ ] todo\n- [ ] done");
  });
});
