import type { Break, ListItem, Paragraph, Root, RootContent, Blockquote } from "mdast";
import type { Plugin } from "unified";

function blankLineParagraph(count: number): Paragraph {
  return {
    type: "paragraph",
    children: Array.from({ length: count }, () => ({ type: "break" }) as Break),
  };
}

function insertBlankLines<T extends RootContent>(children: T[]): T[] {
  const result: RootContent[] = [];
  let previousEndLine: number | undefined;

  for (const child of children) {
    const startLine = child.position?.start?.line;
    if (previousEndLine && startLine && startLine > previousEndLine + 1) {
      result.push(blankLineParagraph(startLine - previousEndLine - 1));
    }
    result.push(child);
    const endLine = child.position?.end?.line;
    if (endLine) previousEndLine = endLine;
  }

  return result as T[];
}

const remarkBlankLines: Plugin<[], Root> = () => (tree) => {
  const visitContainer = (node: Root | Blockquote | ListItem): void => {
    node.children = insertBlankLines(node.children);

    for (const child of node.children) {
      if (child.type === "blockquote" || child.type === "listItem") {
        visitContainer(child);
      } else if (child.type === "list") {
        child.children.forEach(visitContainer);
      }
    }
  };

  visitContainer(tree);
};

export default remarkBlankLines;
