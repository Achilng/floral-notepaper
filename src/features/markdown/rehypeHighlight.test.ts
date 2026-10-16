import { describe, expect, it } from "vitest";
import type { Element, ElementContent, Root, Text } from "hast";
import { HIGHLIGHT_CLASS, highlightTree } from "./rehypeHighlight";

function text(value: string): Text {
  return { type: "text", value };
}

function element(tagName: string, children: ElementContent[]): Element {
  return { type: "element", tagName, properties: {}, children };
}

function paragraph(children: ElementContent[]): Element {
  return element("p", children);
}

function root(children: ElementContent[]): Root {
  return { type: "root", children };
}

function mark(children: ElementContent[]): Element {
  return {
    type: "element",
    tagName: "mark",
    properties: { className: [HIGHLIGHT_CLASS] },
    children,
  };
}

describe("rehypeHighlight", () => {
  it("renders ==text== inside a text node as a mark element", () => {
    const tree = root([paragraph([text("前 ==重点== 后")])]);

    highlightTree(tree);

    expect(tree.children).toEqual([paragraph([text("前 "), mark([text("重点")]), text(" 后")])]);
  });

  it("converts every pair in the same text node", () => {
    const tree = root([paragraph([text("==a== 和 ==b==")])]);

    highlightTree(tree);

    expect(tree.children).toEqual([
      paragraph([mark([text("a")]), text(" 和 "), mark([text("b")])]),
    ]);
  });

  it("keeps spaces inside the highlighted content", () => {
    const tree = root([paragraph([text("==重要 事项==")])]);

    highlightTree(tree);

    expect(tree.children).toEqual([paragraph([mark([text("重要 事项")])])]);
  });

  it("leaves markers spaced away from the content untouched", () => {
    const tree = root([paragraph([text("a == b == c")])]);

    highlightTree(tree);

    expect(tree.children).toEqual([paragraph([text("a == b == c")])]);
  });

  it("leaves odd and empty markers untouched", () => {
    const tree = root([paragraph([text("a == b")]), paragraph([text("====")])]);

    highlightTree(tree);

    expect(tree.children).toEqual([paragraph([text("a == b")]), paragraph([text("====")])]);
  });

  it("wraps inline elements between markers in place", () => {
    const tree = root([paragraph([text("==a "), element("strong", [text("b")]), text(" c==")])]);

    highlightTree(tree);

    expect(tree.children).toEqual([
      paragraph([mark([text("a "), element("strong", [text("b")]), text(" c")])]),
    ]);
  });

  it("wraps a marker pair that surrounds a single inline element", () => {
    const tree = root([paragraph([text("=="), element("strong", [text("全部")]), text("==")])]);

    highlightTree(tree);

    expect(tree.children).toEqual([paragraph([mark([element("strong", [text("全部")])])])]);
  });

  it("does not pair markers across inline code", () => {
    const tree = root([paragraph([text("==a "), element("code", [text("x")]), text(" b==")])]);

    highlightTree(tree);

    expect(tree.children).toEqual([
      paragraph([text("==a "), element("code", [text("x")]), text(" b==")]),
    ]);
  });

  it("keeps markers inside code blocks literal", () => {
    const tree = root([element("pre", [element("code", [text("==x==")])])]);

    highlightTree(tree);

    expect(tree.children).toEqual([element("pre", [element("code", [text("==x==")])])]);
  });

  it("processes nested inline elements independently", () => {
    const tree = root([paragraph([text("=="), element("em", [text("==x==")])])]);

    highlightTree(tree);

    expect(tree.children).toEqual([paragraph([text("=="), element("em", [mark([text("x")])])])]);
  });

  it("does not pair markers across block boundaries", () => {
    const tree = root([paragraph([text("==a")]), paragraph([text("b==")])]);

    highlightTree(tree);

    expect(tree.children).toEqual([paragraph([text("==a")]), paragraph([text("b==")])]);
  });
});
