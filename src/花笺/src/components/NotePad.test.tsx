import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, test } from "vitest";
import { NotePad } from "./NotePad";

describe("NotePad surface modes", () => {
  test("renders the default small window as an editable pad", () => {
    const markup = renderToStaticMarkup(<NotePad />);

    expect(markup).toContain('data-surface-mode="pad"');
    expect(markup).toContain("bg-transparent p-1");
    expect(markup).toContain("border-paper-deep/40 rounded-xl");
    expect(markup).toContain("<input");
    expect(markup).toContain("<textarea");
  });

  test("can render the same surface as the confirmed read-only tile design", () => {
    const markup = renderToStaticMarkup(
      <NotePad initialNoteId="note-1" initialSurfaceMode="tile" />,
    );

    expect(markup).toContain('data-surface-mode="tile"');
    expect(markup).toContain("bg-transparent p-0");
    expect(markup).not.toContain("bg-transparent p-1");
    expect(markup).toContain("rounded-xl");
    expect(markup).toContain("bg-[#d8eee9]");
    expect(markup).toContain("border-[#99cbc1]/55");
    expect(markup).toContain("shadow-[0_1px_8px_rgba(26,26,24,0.04)]");
    expect(markup).toContain('data-tile-corner-mark="true"');
    expect(markup.match(/data-tile-corner-mark="true"/g)).toHaveLength(4);
    expect(markup).not.toContain("<button");
    expect(markup).not.toContain("bg-bamboo-mist/70 p-2");
    expect(markup).not.toContain("<input");
    expect(markup).not.toContain("<textarea");
    expect(markup).not.toContain(">保存<");
    expect(markup).toContain(">空<");
  });
});
