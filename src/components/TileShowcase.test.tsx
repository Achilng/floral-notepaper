import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, test } from "vitest";
import { TileShowcase } from "./TileShowcase";

describe("TileShowcase", () => {
  test("renders the tile as a read-only surface without editing controls", () => {
    const markup = renderToStaticMarkup(<TileShowcase noteId="note-1" />);

    expect(markup).not.toContain("<button");
    expect(markup).not.toContain("<input");
    expect(markup).not.toContain("<textarea");
    expect(markup).not.toContain(">保存<");
  });

  test("keeps the tile chrome minimal and hides metadata labels", () => {
    const markup = renderToStaticMarkup(<TileShowcase noteId="note-1" />);

    expect(markup).not.toContain("无标题磁贴");
    expect(markup).not.toContain("加载中");
    expect(markup).not.toContain("字");
    expect(markup).not.toContain("置顶");
    expect(markup).not.toContain("关闭");
    expect(markup).not.toContain("调整大小");
    expect(markup).toContain('data-tile-corner-mark="true"');
    expect(markup.match(/data-tile-corner-mark="true"/g)).toHaveLength(4);
    expect(markup).toContain(">空<");
  });
});
