import type { Element, ElementContent, Root, RootContent, Text } from "hast";
import type { Plugin } from "unified";

// `==text==` 高亮：Obsidian、Pandoc、iA Writer 等扩展方言通行的写法，渲染为
// <mark>。与项目其他内联语法处理一致，本插件只改写语法树，不重新序列化
// Markdown，避免影响缩进、换行和其他格式。
export const HIGHLIGHT_CLASS = "markdown-highlight";

// 标记必须与内容紧贴，避免把 “a == b” 这类比较写法误判为高亮；空标记
// （====）不匹配。私有区字符 \uE000 为块级边界占位符、\uE001 为内联元素
// 占位符，二者都不参与标记字符。
const HIGHLIGHT_PATTERN = /==([^\s=\uE000](?:[^=\uE000]*[^\s=\uE000])?)==/;

// 代码块、脚本与已生成的高亮内部不做处理
const SKIP_TAGS = new Set(["code", "pre", "script", "style", "mark"]);

// 内联元素：可以整体移入 <mark>，在配对扫描中各占一个占位符；
// 其余元素（块级、未知标签）一律视为边界，打断配对。
const INLINE_TAGS = new Set([
  "a",
  "abbr",
  "b",
  "bdi",
  "bdo",
  "br",
  "cite",
  "data",
  "del",
  "dfn",
  "em",
  "font",
  "i",
  "img",
  "ins",
  "kbd",
  "q",
  "rp",
  "rt",
  "ruby",
  "s",
  "samp",
  "small",
  "span",
  "strong",
  "sub",
  "sup",
  "time",
  "u",
  "var",
  "wbr",
]);

const BOUNDARY_CHAR = "\uE000";
const INLINE_CHAR = "\uE001";

interface Marker {
  node: Text;
  offset: number;
}

type ListChild = RootContent | ElementContent;

function createMark(children: ElementContent[]): Element {
  return {
    type: "element",
    tagName: "mark",
    properties: { className: [HIGHLIGHT_CLASS] },
    children,
  };
}

function createText(value: string): Text {
  return { type: "text", value };
}

// 把一个子节点列表摊平成配对扫描用的虚拟文本：文本节点逐字符登记归属，
// 内联元素占一个占位符，其余节点占一个边界符。
function flattenList(list: ListChild[]): { text: string; owners: Array<Marker | null> } {
  let text = "";
  const owners: Array<Marker | null> = [];

  for (const child of list) {
    if (child.type === "text") {
      for (let index = 0; index < child.value.length; index += 1) {
        text += child.value[index];
        owners.push({ node: child, offset: index });
      }
      continue;
    }

    const isInline = child.type === "element" && INLINE_TAGS.has(child.tagName);
    text += isInline ? INLINE_CHAR : BOUNDARY_CHAR;
    owners.push(null);
  }

  return { text, owners };
}

// 在列表中寻找第一对可安全改写的标记：两个 `==` 必须完整落在同一个文本节点内，
// 否则跳过，保持源码原样，避免生成跨越节点的非法标记。
function findPair(list: ListChild[]): { open: Marker; close: Marker } | null {
  const { text, owners } = flattenList(list);
  const pattern = new RegExp(HIGHLIGHT_PATTERN.source, "g");
  let match: RegExpExecArray | null;

  while ((match = pattern.exec(text)) !== null) {
    const openStart = match.index;
    const closeStart = openStart + match[0].length - 2;
    const open = owners[openStart];
    const openNext = owners[openStart + 1];
    const close = owners[closeStart];
    const closeNext = owners[closeStart + 1];

    if (!open || !openNext || !close || !closeNext) continue;
    if (open.node !== openNext.node || close.node !== closeNext.node) continue;

    return { open, close };
  }

  return null;
}

// 去掉一对标记并把中间内容包进 <mark>：两个标记在同一文本节点内时直接拆分，
// 否则把标记之间的兄弟节点整体移入 <mark>，保留其中的行内格式。
function applyPair(list: ListChild[], pair: { open: Marker; close: Marker }): void {
  const { open, close } = pair;

  if (open.node === close.node) {
    const value = open.node.value;
    const index = list.indexOf(open.node);
    const replacement: ListChild[] = [];
    const before = value.slice(0, open.offset);
    const content = value.slice(open.offset + 2, close.offset);
    const after = value.slice(close.offset + 2);

    if (before) replacement.push(createText(before));
    replacement.push(createMark([createText(content)]));
    if (after) replacement.push(createText(after));
    list.splice(index, 1, ...replacement);
    return;
  }

  const openIndex = list.indexOf(open.node);
  const closeIndex = list.indexOf(close.node);
  const before = open.node.value.slice(0, open.offset);
  const openRemainder = open.node.value.slice(open.offset + 2);
  const closeRemainder = close.node.value.slice(0, close.offset);
  const after = close.node.value.slice(close.offset + 2);
  const children: ElementContent[] = [];

  if (openRemainder) children.push(createText(openRemainder));
  children.push(...(list.slice(openIndex + 1, closeIndex) as ElementContent[]));
  if (closeRemainder) children.push(createText(closeRemainder));

  const replacement: ListChild[] = [];
  if (before) replacement.push(createText(before));
  replacement.push(createMark(children));
  if (after) replacement.push(createText(after));
  list.splice(openIndex, closeIndex - openIndex + 1, ...replacement);
}

function processList(list: ListChild[]): void {
  for (const child of list) {
    if (child.type === "element" && !SKIP_TAGS.has(child.tagName)) {
      processList(child.children);
    }
  }

  // 每轮重新扫描并消解一对标记，直到列表内不再有可配对的标记；
  // 生成的高亮节点视为边界，不会再次参与配对。
  for (let pair = findPair(list); pair !== null; pair = findPair(list)) {
    applyPair(list, pair);
  }
}

export function highlightTree(tree: Root): void {
  processList(tree.children);
}

const rehypeHighlight: Plugin<[], Root> = () => (tree) => {
  highlightTree(tree);
};

export default rehypeHighlight;
