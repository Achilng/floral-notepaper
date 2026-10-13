import { describe, expect, test } from "vitest";
import { shouldRenderTileMarkdown } from "./tileContent";

describe("shouldRenderTileMarkdown", () => {
  test("honors the tile Markdown setting", () => {
    expect(shouldRenderTileMarkdown("**正文**", true)).toBe(true);
    expect(shouldRenderTileMarkdown("**正文**", false)).toBe(false);
  });

  test("renders pasted note images even when tile Markdown is disabled", () => {
    expect(shouldRenderTileMarkdown("正文\n![](images/photo.png)", false)).toBe(true);
    expect(shouldRenderTileMarkdown("![截图](images/screenshot.webp)", false)).toBe(true);
  });

  test("does not treat normal links or malformed image text as images", () => {
    expect(shouldRenderTileMarkdown("[图片](images/photo.png)", false)).toBe(false);
    expect(shouldRenderTileMarkdown("![] images/photo.png", false)).toBe(false);
  });
});
