import { afterEach, describe, expect, test, vi } from "vitest";
import {
  createScrollSyncMap,
  interpolateScrollOffset,
  measureBlockOffsets,
  measurePreviewBlockOffsets,
  tagPreviewBlocks,
} from "./scrollSync";

class FakeTextNode {
  constructor(public readonly textContent: string) {}
}

class FakeStyle {
  cssText = "";
}

class FakeHTMLElement {
  static onOffsetTopRead: ((element: FakeHTMLElement) => void) | undefined;
  static offsetTopReads = 0;
  static scrollHeightReads = 0;

  readonly style = new FakeStyle();
  readonly children: FakeHTMLElement[] = [];
  readonly attributes = new Map<string, string>();
  className = "";
  clientWidth = 240;
  parentNode: FakeHTMLElement | null = null;
  private line = 0;
  private top = 0;

  constructor(public readonly tagName = "div") {}

  appendChild<T>(child: T): T {
    if (child instanceof FakeTextNode) {
      this.line += child.textContent.split("\n").length - 1;
      return child;
    }

    if (child instanceof FakeHTMLElement) {
      child.parentNode = this;
      if (child.tagName === "span") child.top = this.paddingTop + this.line * this.lineHeight;
      this.children.push(child);
    }
    return child;
  }

  removeChild<T>(child: T): T {
    if (child instanceof FakeHTMLElement) {
      const index = this.children.indexOf(child);
      if (index >= 0) this.children.splice(index, 1);
      child.parentNode = null;
    }
    return child;
  }

  setAttribute(name: string, value: string): void {
    this.attributes.set(name, value);
  }

  getAttribute(name: string): string | null {
    return this.attributes.get(name) ?? null;
  }

  querySelector<T>(selector: string): T | null {
    if (selector !== ".font-body") return null;
    return this.findByClass("font-body") as T | null;
  }

  get offsetTop(): number {
    FakeHTMLElement.offsetTopReads++;
    FakeHTMLElement.onOffsetTopRead?.(this);
    return this.top;
  }

  get scrollHeight(): number {
    FakeHTMLElement.scrollHeightReads++;
    return 0;
  }

  private findByClass(className: string): FakeHTMLElement | null {
    if (this.className.split(/\s+/).includes(className)) return this;
    for (const child of this.children) {
      const found = child.findByClass(className);
      if (found) return found;
    }
    return null;
  }

  private get lineHeight(): number {
    return this.numberFromStyle("line-height") ?? 20;
  }

  private get paddingTop(): number {
    return this.numberFromStyle("padding-top") ?? 0;
  }

  private numberFromStyle(property: string): number | undefined {
    const match = this.style.cssText.match(new RegExp(`${property}:\\s*([^;]+)`));
    if (!match) return undefined;
    const parsed = parseFloat(match[1]);
    return Number.isFinite(parsed) ? parsed : undefined;
  }
}

interface FakeDocument {
  body: FakeHTMLElement;
  createdElements: FakeHTMLElement[];
  createElement(tagName: string): FakeHTMLElement;
  createTextNode(text: string): FakeTextNode;
}

const originalDocument = globalThis.document;
const originalHTMLElement = globalThis.HTMLElement;
const originalGetComputedStyle = globalThis.getComputedStyle;

function installFakeDom(): FakeDocument {
  FakeHTMLElement.onOffsetTopRead = undefined;
  FakeHTMLElement.offsetTopReads = 0;
  FakeHTMLElement.scrollHeightReads = 0;

  const fakeDocument: FakeDocument = {
    body: new FakeHTMLElement("body"),
    createdElements: [],
    createElement(tagName: string) {
      const element = new FakeHTMLElement(tagName);
      this.createdElements.push(element);
      return element;
    },
    createTextNode(text: string) {
      return new FakeTextNode(text);
    },
  };

  vi.stubGlobal("document", fakeDocument);
  vi.stubGlobal("HTMLElement", FakeHTMLElement);
  vi.stubGlobal("getComputedStyle", () => ({
    width: "200px",
    boxSizing: "border-box",
    font: "16px sans-serif",
    fontSize: "16px",
    fontFamily: "sans-serif",
    fontWeight: "400",
    fontStyle: "normal",
    lineHeight: "18px",
    letterSpacing: "0px",
    wordSpacing: "0px",
    wordBreak: "normal",
    overflowWrap: "break-word",
    wordWrap: "break-word",
    tabSize: "4",
    padding: "4px 8px",
    paddingTop: "4px",
    paddingRight: "8px",
    paddingBottom: "4px",
    paddingLeft: "8px",
    border: "0",
    borderTop: "0",
    borderRight: "0",
    borderBottom: "0",
    borderLeft: "0",
  }));

  return fakeDocument;
}

afterEach(() => {
  vi.unstubAllGlobals();
  if (originalDocument) vi.stubGlobal("document", originalDocument);
  if (originalHTMLElement) vi.stubGlobal("HTMLElement", originalHTMLElement);
  if (originalGetComputedStyle) vi.stubGlobal("getComputedStyle", originalGetComputedStyle);
});

describe("scroll sync helpers", () => {
  test("builds a monotonic map with fixed document boundaries", () => {
    const map = createScrollSyncMap([0, 100, 300, 500], [12, 220, 260, 900], 400, 500);

    expect(map.source).toEqual([0, 100, 300, 400]);
    expect(map.target).toEqual([0, 220, 260, 500]);
  });

  test("interpolates continuously between block anchors and clamps at both ends", () => {
    const map = createScrollSyncMap([0, 100, 300], [0, 220, 260], 400, 500);

    expect(interpolateScrollOffset(map, -10)).toBe(0);
    expect(interpolateScrollOffset(map, 50)).toBe(110);
    expect(interpolateScrollOffset(map, 200)).toBe(240);
    expect(interpolateScrollOffset(map, 350)).toBe(380);
    expect(interpolateScrollOffset(map, 999)).toBe(500);
  });

  test("coalesces duplicate anchors and ignores invalid or out-of-order offsets", () => {
    const map = createScrollSyncMap(
      [0, 50, 50, 40, 100, Number.NaN],
      [0, 20, 30, 35, 80, 90],
      120,
      100,
    );

    expect(map.source).toEqual([0, 50, 100, 120]);
    expect(map.target).toEqual([0, 30, 80, 100]);
  });

  test("falls back to proportional scrolling when there are no usable block anchors", () => {
    const map = createScrollSyncMap([], [], 100, 250);

    expect(interpolateScrollOffset(map, 40)).toBe(100);
    expect(interpolateScrollOffset(createScrollSyncMap([], [], 0, 250), 40)).toBe(0);
  });

  test("handles large anchor maps without scanning every block per scroll", () => {
    const source = Array.from({ length: 10_000 }, (_, index) => index * 10);
    const target = Array.from({ length: 10_000 }, (_, index) => index * 20);
    const map = createScrollSyncMap(source, target, 100_000, 200_000);

    expect(interpolateScrollOffset(map, 55_555)).toBe(111_110);
  });

  test("tags preview block children with sequential block indices", () => {
    installFakeDom();
    const container = new FakeHTMLElement("section");
    const root = new FakeHTMLElement("article");
    root.className = "font-body";
    const heading = new FakeHTMLElement("h1");
    const paragraph = new FakeHTMLElement("p");
    const list = new FakeHTMLElement("ul");

    container.appendChild(root);
    root.appendChild(heading);
    root.appendChild(paragraph);
    root.appendChild(list);

    tagPreviewBlocks(container as unknown as HTMLElement);

    expect(heading.getAttribute("data-block-index")).toBe("0");
    expect(paragraph.getAttribute("data-block-index")).toBe("1");
    expect(list.getAttribute("data-block-index")).toBe("2");
  });

  test("measures preview blocks in the scroll container coordinate space", () => {
    const elements = [
      {
        dataset: { blockIndex: "0" },
        getBoundingClientRect: () => ({ top: 120 }),
      },
      {
        dataset: { blockIndex: "1" },
        getBoundingClientRect: () => ({ top: 180 }),
      },
    ];
    const container = {
      scrollTop: 30,
      clientTop: 2,
      getBoundingClientRect: () => ({ top: 100 }),
      querySelectorAll: () => elements,
    } as unknown as HTMLElement;

    expect(measurePreviewBlockOffsets(container)).toEqual([48, 108]);
  });

  test("measures representative markdown blocks from their source line starts", async () => {
    installFakeDom();
    const source = new FakeHTMLElement("textarea") as unknown as HTMLTextAreaElement;
    const content = [
      "# Title",
      "",
      "Paragraph line",
      "wrapped second",
      "",
      "- first",
      "- second",
      "",
      "```ts",
      "const value = 1;",
      "```",
    ].join("\n");

    await expect(measureBlockOffsets(content, source)).resolves.toEqual([0, 36, 90, 144]);
  });

  test("skips standalone markdown link reference definitions before visible blocks", async () => {
    installFakeDom();
    const source = new FakeHTMLElement("textarea") as unknown as HTMLTextAreaElement;
    const content = [
      "# Title",
      "",
      '[docs]: https://example.com/docs "Documentation"',
      "[api]: /api",
      "",
      "Paragraph with a [docs] link.",
      "",
      "- first",
      "- second",
    ].join("\n");

    await expect(measureBlockOffsets(content, source)).resolves.toEqual([0, 90, 126]);
  });

  test("keeps paragraph reference-shaped lines in the visible paragraph block", async () => {
    installFakeDom();
    const source = new FakeHTMLElement("textarea") as unknown as HTMLTextAreaElement;
    const content = [
      "# Title",
      "",
      "Paragraph start",
      "[visible]: https://example.com/visible",
      "Paragraph end",
      "",
      "## Next",
    ].join("\n");

    await expect(measureBlockOffsets(content, source)).resolves.toEqual([0, 36, 108]);
  });

  test("keeps list reference-shaped lines in the visible list block", async () => {
    installFakeDom();
    const source = new FakeHTMLElement("textarea") as unknown as HTMLTextAreaElement;
    const content = [
      "# Title",
      "",
      "- first",
      "[visible-list]: https://example.com/list",
      "- second",
      "",
      "## Next",
    ].join("\n");

    await expect(measureBlockOffsets(content, source)).resolves.toEqual([0, 36, 108]);
  });

  test("cleans up its hidden mirror and returns no offsets when already aborted", async () => {
    const fakeDocument = installFakeDom();
    const controller = new AbortController();
    controller.abort();

    const offsets = await measureBlockOffsets(
      "# Title\n\nParagraph",
      new FakeHTMLElement("textarea") as unknown as HTMLTextAreaElement,
      controller.signal,
    );

    expect(offsets).toEqual([]);
    expect(fakeDocument.body.children).toHaveLength(0);
  });

  test("cleans up its hidden mirror and may return partial offsets when aborted mid-measurement", async () => {
    const fakeDocument = installFakeDom();
    const controller = new AbortController();
    FakeHTMLElement.onOffsetTopRead = () => {
      if (FakeHTMLElement.offsetTopReads === 2) controller.abort();
    };

    const offsets = await measureBlockOffsets(
      "# One\n\n# Two\n\n# Three\n\n# Four",
      new FakeHTMLElement("textarea") as unknown as HTMLTextAreaElement,
      controller.signal,
    );

    expect(offsets).toEqual([0, 36]);
    expect(fakeDocument.body.children).toHaveLength(0);
  });

  test("yields during long marker measurements so async aborts can cancel and clean up", async () => {
    const fakeDocument = installFakeDom();
    const controller = new AbortController();
    const content = Array.from({ length: 130 }, (_, index) => `# Heading ${index}`).join("\n\n");

    setTimeout(() => controller.abort(), 0);

    const offsets = await measureBlockOffsets(
      content,
      new FakeHTMLElement("textarea") as unknown as HTMLTextAreaElement,
      controller.signal,
    );

    expect(offsets.length).toBeGreaterThan(0);
    expect(offsets.length).toBeLessThan(130);
    expect(FakeHTMLElement.offsetTopReads).toBe(offsets.length);
    expect(fakeDocument.body.children).toHaveLength(0);
  });

  test("yields during huge mirror construction so async aborts can cancel before attachment", async () => {
    const fakeDocument = installFakeDom();
    const controller = new AbortController();
    const content = Array.from({ length: 300 }, (_, index) => `# Heading ${index}`).join("\n\n");

    setTimeout(() => controller.abort(), 0);

    const offsets = await measureBlockOffsets(
      content,
      new FakeHTMLElement("textarea") as unknown as HTMLTextAreaElement,
      controller.signal,
    );

    expect(offsets).toEqual([]);
    expect(FakeHTMLElement.offsetTopReads).toBe(0);
    expect(fakeDocument.body.children).toHaveLength(0);
  });

  test("does not remeasure growing textarea prefixes for long documents", async () => {
    const fakeDocument = installFakeDom();
    const content = Array.from(
      { length: 120 },
      (_, index) => `# Heading ${index}\n\nParagraph ${index}`,
    ).join("\n\n");

    const offsets = await measureBlockOffsets(
      content,
      new FakeHTMLElement("textarea") as unknown as HTMLTextAreaElement,
    );

    expect(offsets).toHaveLength(240);
    expect(
      fakeDocument.createdElements.filter((element) => element.tagName === "textarea"),
    ).toHaveLength(0);
    expect(FakeHTMLElement.scrollHeightReads).toBe(0);
  });
});
