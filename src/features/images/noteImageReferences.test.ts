import { describe, expect, test } from "vitest";
import {
  extractNoteImageReferences,
  parseNoteContentParts,
  visibleNoteText,
} from "./noteImageReferences";

describe("extractNoteImageReferences", () => {
  test("extracts app-managed note images in content order", () => {
    expect(
      extractNoteImageReferences(
        ["before", "![](images/note-1/first.png)", "![diagram](images/note-1/second.webp)"].join(
          "\n",
        ),
      ),
    ).toEqual([
      { alt: "", src: "images/note-1/first.png" },
      { alt: "diagram", src: "images/note-1/second.webp" },
    ]);
  });

  test("accepts Windows separators and optional Markdown titles", () => {
    expect(
      extractNoteImageReferences('![photo](images\\note-1\\photo.jpg "camera upload")'),
    ).toEqual([{ alt: "photo", src: "images\\note-1\\photo.jpg" }]);
  });

  test("ignores remote images, code examples, inline prose, and traversal paths", () => {
    const content = [
      "![](https://example.com/remote.png)",
      "text ![](images/note-1/inline.png)",
      "    ![](images/note-1/indented-code.png)",
      "![](images/note-1/../../private.png)",
      "```markdown",
      "![](images/note-1/example.png)",
      "```",
    ].join("\n");

    expect(extractNoteImageReferences(content)).toEqual([]);
  });

  test("does not close a fenced code block on a fence-like content line", () => {
    const content = [
      "```markdown",
      "```not-a-closing-fence",
      "![](images/note-1/example.png)",
      "```",
    ].join("\n");

    expect(extractNoteImageReferences(content)).toEqual([]);
  });

  test("keeps canonical offsets while replacing image lines with ordered parts", () => {
    const content = "before\n![](images/note-1/photo.png)\nafter";
    const parts = parseNoteContentParts(content);

    expect(parts).toEqual([
      { type: "text", value: "before", start: 0, end: 6 },
      {
        type: "image",
        raw: "\n![](images/note-1/photo.png)\n",
        reference: { alt: "", src: "images/note-1/photo.png" },
        start: 6,
        end: 36,
      },
      { type: "text", value: "after", start: 36, end: 41 },
    ]);
    expect(parts.map((part) => (part.type === "text" ? part.value : part.raw)).join("")).toBe(
      content,
    );
    expect(visibleNoteText(content)).toBe("beforeafter");
  });
});
