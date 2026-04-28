import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, test } from "vitest";
import { Tile } from "./Tile";

describe("Tile", () => {
  test("renders the confirmed tile design structure", () => {
    const markup = renderToStaticMarkup(
      <Tile title="读书笔记" content={"满地都是六便士，\n他却抬头看见了月亮。"} />,
    );

    expect(markup).toContain("rounded-xl");
    expect(markup).toContain("bg-cloud");
    expect(markup).toContain("border-paper-deep/30");
    expect(markup).toContain("shadow-[0_1px_8px_rgba(26,26,24,0.04)]");
    expect(markup).toContain("hover:shadow-[0_6px_24px_rgba(26,26,24,0.07)]");
    expect(markup).toContain("hover:scale-[1.008]");
    expect(markup).toContain("leading-[1.8]");
    expect(markup).toContain(">读书笔记<");
    expect(markup).toContain("满地都是六便士");
    expect(markup.match(/data-tile-corner-mark="true"/g)).toHaveLength(4);
  });

  test("renders the same empty state as the design draft", () => {
    const markup = renderToStaticMarkup(<Tile content="" />);

    expect(markup).toContain(">空<");
    expect(markup).toContain("text-ink-ghost/40");
  });

  test("supports a deeper cyan tile color for pinned tiles", () => {
    const markup = renderToStaticMarkup(<Tile color="cyan" content="磁贴内容" />);

    expect(markup).toContain("bg-[#d8eee9]");
    expect(markup).toContain("border-[#99cbc1]/55");
  });
});
