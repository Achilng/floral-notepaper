import { describe, expect, it } from "vitest";
import remarkHighlight from "./remarkHighlight";
import type { Root } from "mdast";

function processText(text: string): Array<{ type: string; value?: string }> {
  const tree: Root = {
    type: "root",
    children: [
      {
        type: "paragraph",
        children: [{ type: "text", value: text }],
      },
    ],
  };

  const transformer = remarkHighlight.call(undefined as any) as unknown as (
    tree: Root,
  ) => void;
  transformer(tree);

  return (tree.children[0] as any).children;
}

describe("remarkHighlight", () => {
  it("leaves plain text unchanged", () => {
    const nodes = processText("hello world");
    expect(nodes).toEqual([{ type: "text", value: "hello world" }]);
  });

  it("converts a single highlight into a mark html node", () => {
    const nodes = processText("==hello==");
    expect(nodes).toEqual([{ type: "html", value: "<mark>hello</mark>" }]);
  });

  it("handles highlight surrounded by plain text", () => {
    const nodes = processText("before ==mid== after");
    expect(nodes).toEqual([
      { type: "text", value: "before " },
      { type: "html", value: "<mark>mid</mark>" },
      { type: "text", value: " after" },
    ]);
  });

  it("handles multiple highlights in one string", () => {
    const nodes = processText("==a== and ==b==");
    expect(nodes).toEqual([
      { type: "html", value: "<mark>a</mark>" },
      { type: "text", value: " and " },
      { type: "html", value: "<mark>b</mark>" },
    ]);
  });

  it("does not match empty highlights", () => {
    const nodes = processText("====");
    expect(nodes).toEqual([{ type: "text", value: "====" }]);
  });

  it("does not match unclosed markers", () => {
    const nodes = processText("==unclosed text");
    expect(nodes).toEqual([{ type: "text", value: "==unclosed text" }]);
  });

  it("handles equals signs inside highlighted content", () => {
    const nodes = processText("==a=b==");
    expect(nodes).toEqual([{ type: "html", value: "<mark>a=b</mark>" }]);
  });

  it("preserves trailing unclosed markers as plain text", () => {
    const nodes = processText("==ok== and ==no");
    expect(nodes).toEqual([
      { type: "html", value: "<mark>ok</mark>" },
      { type: "text", value: " and ==no" },
    ]);
  });

  it("returns single text node for empty string", () => {
    const nodes = processText("");
    expect(nodes).toEqual([{ type: "text", value: "" }]);
  });

  it("handles highlight at the start of text", () => {
    const nodes = processText("==start== rest");
    expect(nodes).toEqual([
      { type: "html", value: "<mark>start</mark>" },
      { type: "text", value: " rest" },
    ]);
  });

  it("handles highlight at the end of text", () => {
    const nodes = processText("rest ==end==");
    expect(nodes).toEqual([
      { type: "text", value: "rest " },
      { type: "html", value: "<mark>end</mark>" },
    ]);
  });

  it("handles Chinese text highlights", () => {
    const nodes = processText("这是==高亮文本==的示例");
    expect(nodes).toEqual([
      { type: "text", value: "这是" },
      { type: "html", value: "<mark>高亮文本</mark>" },
      { type: "text", value: "的示例" },
    ]);
  });

  it("handles multiple Chinese highlights", () => {
    const nodes = processText("这是一段==高亮==和另一段==强调==文本");
    expect(nodes).toEqual([
      { type: "text", value: "这是一段" },
      { type: "html", value: "<mark>高亮</mark>" },
      { type: "text", value: "和另一段" },
      { type: "html", value: "<mark>强调</mark>" },
      { type: "text", value: "文本" },
    ]);
  });
});
