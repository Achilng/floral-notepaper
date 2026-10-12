import type { Root } from "hast";
import type { Plugin } from "unified";
import { visit } from "unist-util-visit";

// GFM 已负责识别任务项；这里只利用语法树的位置找回原文中的勾选字符，
// 不重新序列化 Markdown，避免改变缩进、换行和其他格式。
const rehypeTaskLists: Plugin<[], Root> = () => (tree, file) => {
  const source = String(file.value);
  visit(tree, "element", (node) => {
    if (
      node.tagName !== "li" ||
      !node.properties.className?.toString().includes("task-list-item")
    ) {
      return;
    }
    const start = node.position?.start.offset;
    if (start == null) return;

    const marker = source
      .slice(start, node.position?.end.offset)
      .match(/^(?:[-+*]|\d+[.)])[\t >\r\n]*\[[ xX]\]/);
    if (!marker) return;

    const first = node.children.find((child) => child.type === "element");
    const input =
      first?.type === "element" && first.tagName === "p"
        ? first.children.find((child) => child.type === "element")
        : first;
    if (
      input?.type !== "element" ||
      input.tagName !== "input" ||
      input.properties.type !== "checkbox"
    ) {
      return;
    }
    input.properties.dataTaskOffset = start + marker[0].length - 2;
    input.properties.dataTaskLabel = source
      .slice(start + marker[0].length)
      .split(/\r?\n/, 1)[0]
      .trim();
  });
};

export default rehypeTaskLists;
