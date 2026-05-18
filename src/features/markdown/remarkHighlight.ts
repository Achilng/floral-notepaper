import { visit } from "unist-util-visit";
import type { Plugin } from "unified";
import type { Text } from "mdast";

const remarkHighlight: Plugin = () => (tree) => {
  visit(tree, "text", (node: Text, index, parent) => {
    if (index === null || !parent || !node.value.includes("==")) return;

    const regex = /==((?:[^=]|=(?!=))+)==/g;
    let lastIndex = 0;
    let hasMatch = false;
    const newNodes: any[] = [];

    let match;
    while ((match = regex.exec(node.value)) !== null) {
      hasMatch = true;
      if (match.index > lastIndex) {
        newNodes.push({ type: "text", value: node.value.slice(lastIndex, match.index) });
      }
      newNodes.push({ type: "html", value: `<mark>${match[1]}</mark>` });
      lastIndex = match.index + match[0].length;
    }

    if (lastIndex < node.value.length) {
      newNodes.push({ type: "text", value: node.value.slice(lastIndex) });
    }

    if (hasMatch) {
      (parent as any).children.splice(index, 1, ...newNodes);
    }
  });
};

export default remarkHighlight;
