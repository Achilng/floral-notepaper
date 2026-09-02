import { createRef } from "react";
import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, test, vi } from "vitest";
import { NoteInlineImageEditor } from "./NoteInlineImageEditor";

vi.mock("@tauri-apps/api/core", () => ({
  convertFileSrc: vi.fn((path: string) => `asset://${path}`),
  invoke: vi.fn(),
}));

describe("NoteInlineImageEditor", () => {
  test("renders a managed image in content order without exposing its Markdown", () => {
    const markup = renderToStaticMarkup(
      <NoteInlineImageEditor
        content={"before\n![](images/note-1/photo.png)\nafter"}
        imageBaseDir="C:/data"
        fontSize={14}
        noteId="note-1"
        focusOffset={null}
        primaryTextareaRef={createRef<HTMLTextAreaElement>()}
        setContent={vi.fn()}
        markDirty={vi.fn()}
        onEnsureNoteSaved={vi.fn(async () => "note-1")}
        onImagesInserted={vi.fn()}
        onError={vi.fn()}
        onArrowUpFromStart={vi.fn()}
        onFocusRestored={vi.fn()}
      />,
    );

    const beforeIndex = markup.indexOf(">before</textarea>");
    const imageIndex = markup.indexOf("<img");
    const afterIndex = markup.indexOf(">after</textarea>");

    expect(beforeIndex).toBeGreaterThan(-1);
    expect(imageIndex).toBeGreaterThan(beforeIndex);
    expect(afterIndex).toBeGreaterThan(imageIndex);
    expect(markup).not.toContain("![](");
    expect(markup).toContain('src="asset://C:/data/images/note-1/photo.png"');
  });
});
