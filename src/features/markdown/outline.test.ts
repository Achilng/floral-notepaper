import { describe, expect, test } from "vitest";
import { activeHeadingByLine, parseOutline } from "./outline";

describe("parseOutline", () => {
  test("extracts heading level, text and zero-based line", () => {
    const items = parseOutline("# Title\n\nsome text\n\n## Section\n### Sub");
    expect(items).toEqual([
      { level: 1, text: "Title", slug: "title", line: 0, from: 0 },
      { level: 2, text: "Section", slug: "section", line: 4, from: 20 },
      { level: 3, text: "Sub", slug: "sub", line: 5, from: 31 },
    ]);
  });

  test("ignores headings inside fenced code blocks (``` and ~~~)", () => {
    const md = [
      "# Real",
      "```",
      "# fake in code",
      "```",
      "~~~",
      "## also fake",
      "~~~",
      "## Real2",
    ].join("\n");
    expect(parseOutline(md).map((h) => h.text)).toEqual(["Real", "Real2"]);
  });

  test("requires a space after the hashes and at most six", () => {
    const items = parseOutline("#nothash\n####### too many\n###### Six");
    expect(items.map((h) => h.text)).toEqual(["Six"]);
    expect(items[0].level).toBe(6);
  });

  test("strips inline markdown so the text matches the rendered heading", () => {
    const items = parseOutline("# **Bold** and `code` and [link](http://x)");
    expect(items[0].text).toBe("Bold and code and link");
    expect(items[0].slug).toBe("bold-and-code-and-link");
  });

  test("drops optional closing hashes", () => {
    expect(parseOutline("## Title ##")[0].text).toBe("Title");
  });

  test("disambiguates duplicate headings like rehype-slug", () => {
    const items = parseOutline("# Hello\n# Hello\n# Hello");
    expect(items.map((h) => h.slug)).toEqual(["hello", "hello-1", "hello-2"]);
  });

  test("returns an empty list when there are no headings", () => {
    expect(parseOutline("just a paragraph\nand another line")).toEqual([]);
  });
});

describe("activeHeadingByLine", () => {
  const items = parseOutline("# A\n\ntext\n## B\nmore\n## C");
  // A=line0, B=line3, C=line5

  test("returns the last heading at or above the line", () => {
    expect(activeHeadingByLine(items, 0)).toBe("a");
    expect(activeHeadingByLine(items, 2)).toBe("a");
    expect(activeHeadingByLine(items, 3)).toBe("b");
    expect(activeHeadingByLine(items, 4)).toBe("b");
    expect(activeHeadingByLine(items, 99)).toBe("c");
  });

  test("returns null before the first heading and for empty input", () => {
    const withLead = parseOutline("intro\n# A");
    expect(activeHeadingByLine(withLead, 0)).toBeNull();
    expect(activeHeadingByLine([], 5)).toBeNull();
  });
});
