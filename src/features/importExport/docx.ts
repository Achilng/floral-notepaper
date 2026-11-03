import {
  Document,
  Packer,
  Paragraph,
  TextRun,
  HeadingLevel,
  ExternalHyperlink,
  Table,
  TableRow,
  TableCell,
  WidthType,
  ImageRun,
  LevelFormat,
  AlignmentType,
  type ParagraphChild,
  type IRunOptions,
  type INumberingOptions,
} from "docx";
import { unified } from "unified";
import remarkParse from "remark-parse";
import remarkGfm from "remark-gfm";
import remarkMath from "remark-math";
import type { Root, RootContent, PhrasingContent, Definition } from "mdast";
import { convertFileSrc } from "@tauri-apps/api/core";
import { resolveMarkdownImageSrc } from "../markdown/imageSrc";
import { renderMermaid } from "../markdown/mermaid";

type RunStyle = Pick<IRunOptions, "bold" | "italics" | "strike" | "font" | "color">;

// Rasterize diagrams and images before packaging so Word needs no external files.
async function imageRun(src: string, alt: string): Promise<ImageRun> {
  const image = new Image();
  image.crossOrigin = "anonymous";
  await new Promise<void>((resolve, reject) => {
    const timeout = window.setTimeout(() => {
      image.src = "";
      reject(new Error(`Image load timed out: ${alt || src}`));
    }, 15000);
    image.onload = () => {
      window.clearTimeout(timeout);
      resolve();
    };
    image.onerror = () => {
      window.clearTimeout(timeout);
      reject(new Error(`Image could not be exported: ${alt || src}`));
    };
    image.src = src;
  });
  const scale = Math.min(1, 600 / image.naturalWidth, 800 / image.naturalHeight);
  const width = Math.max(1, Math.round(image.naturalWidth * scale));
  const height = Math.max(1, Math.round(image.naturalHeight * scale));
  const canvas = document.createElement("canvas");
  canvas.width = width * 2;
  canvas.height = height * 2;
  const context = canvas.getContext("2d");
  if (!context) throw new Error("Canvas is unavailable for DOCX export");
  context.fillStyle = "#ffffff";
  context.fillRect(0, 0, canvas.width, canvas.height);
  context.drawImage(image, 0, 0, canvas.width, canvas.height);
  const binary = atob(canvas.toDataURL("image/png").split(",")[1]);
  return new ImageRun({
    type: "png",
    data: Uint8Array.from(binary, (char) => char.charCodeAt(0)),
    transformation: { width, height },
    altText: { name: alt, title: alt, description: alt },
  });
}

export async function markdownToDocx(
  content: string,
  title: string,
  imageBaseDir?: string,
): Promise<Uint8Array> {
  const tree = unified().use(remarkParse).use(remarkGfm).use(remarkMath).parse(content) as Root;
  const definitions = new Map<string, Definition>();
  const collect = (nodes: RootContent[]) => {
    for (const node of nodes) {
      if (node.type === "definition") definitions.set(node.identifier.toLowerCase(), node);
      if ("children" in node) collect(node.children as RootContent[]);
    }
  };
  collect(tree.children);
  const numbering: INumberingOptions["config"][number][] = [];

  async function inline(nodes: PhrasingContent[], style: RunStyle = {}): Promise<ParagraphChild[]> {
    const runs: ParagraphChild[] = [];
    for (const node of nodes) {
      switch (node.type) {
        case "text":
          runs.push(new TextRun({ text: node.value, ...style }));
          break;
        case "strong":
          runs.push(...(await inline(node.children, { ...style, bold: true })));
          break;
        case "emphasis":
          runs.push(...(await inline(node.children, { ...style, italics: true })));
          break;
        case "delete":
          runs.push(...(await inline(node.children, { ...style, strike: true })));
          break;
        case "inlineCode":
        case "inlineMath":
          runs.push(new TextRun({ text: node.value, ...style, font: "Consolas" }));
          break;
        case "break":
          runs.push(new TextRun({ break: 1 }));
          break;
        case "link":
        case "linkReference": {
          const url =
            node.type === "link" ? node.url : definitions.get(node.identifier.toLowerCase())?.url;
          const children = await inline(node.children, { ...style, color: "2D5A3D" });
          runs.push(
            ...(url && /^https?:|^mailto:/i.test(url)
              ? [new ExternalHyperlink({ link: url, children })]
              : children),
          );
          break;
        }
        case "image":
        case "imageReference": {
          const url =
            node.type === "image" ? node.url : definitions.get(node.identifier.toLowerCase())?.url;
          if (url)
            runs.push(
              await imageRun(
                resolveMarkdownImageSrc(url, imageBaseDir, convertFileSrc),
                node.alt ?? "",
              ),
            );
          else runs.push(new TextRun(node.alt ?? ""));
          break;
        }
        case "html":
          runs.push(new TextRun({ text: node.value, ...style }));
          break;
        case "footnoteReference":
          runs.push(new TextRun({ text: `[${node.identifier}]`, ...style }));
          break;
      }
    }
    return runs;
  }

  async function blocks(
    nodes: RootContent[],
    depth = 0,
    quote = false,
  ): Promise<(Paragraph | Table)[]> {
    const result: (Paragraph | Table)[] = [];
    for (const node of nodes) {
      const indent = { left: depth * 360 + (quote ? 360 : 0) };
      switch (node.type) {
        case "heading":
          result.push(
            new Paragraph({
              heading: HeadingLevel[`HEADING_${node.depth}`],
              children: await inline(node.children),
              spacing: { before: 240, after: 120 },
              keepNext: true,
            }),
          );
          break;
        case "paragraph":
          result.push(new Paragraph({ children: await inline(node.children), indent }));
          break;
        case "blockquote":
          result.push(...(await blocks(node.children, depth, true)));
          break;
        case "footnoteDefinition":
          result.push(new Paragraph({ children: [new TextRun(`[${node.identifier}]`)] }));
          result.push(...(await blocks(node.children, depth, quote)));
          break;
        case "list": {
          const reference = `list-${numbering.length}`;
          numbering.push({
            reference,
            levels: Array.from({ length: 9 }, (_, level) => ({
              level,
              format: node.ordered ? LevelFormat.DECIMAL : LevelFormat.BULLET,
              text: node.ordered ? `%${level + 1}.` : "•",
              alignment: AlignmentType.START,
              start: node.start ?? 1,
              style: { paragraph: { indent: { left: (level + 1) * 360, hanging: 240 } } },
            })),
          });
          for (const item of node.children) {
            const firstNode = item.children[0];
            const listParagraph = new Paragraph({
              children: [
                ...(item.checked != null ? [new TextRun(item.checked ? "☑ " : "☐ ")] : []),
                ...(firstNode?.type === "paragraph" ? await inline(firstNode.children) : []),
              ],
              numbering: { reference, level: Math.min(depth, 8) },
            });
            result.push(
              listParagraph,
              ...(await blocks(
                item.children.slice(firstNode?.type === "paragraph" ? 1 : 0),
                depth + 1,
                quote,
              )),
            );
          }
          break;
        }
        case "table":
          result.push(
            new Table({
              width: { size: 100, type: WidthType.PERCENTAGE },
              rows: await Promise.all(
                node.children.map(
                  async (row, rowIndex) =>
                    new TableRow({
                      tableHeader: rowIndex === 0,
                      children: await Promise.all(
                        row.children.map(
                          async (cell, index) =>
                            new TableCell({
                              children: [
                                new Paragraph({
                                  children: await inline(cell.children, { bold: rowIndex === 0 }),
                                  alignment:
                                    node.align?.[index] === "center"
                                      ? AlignmentType.CENTER
                                      : node.align?.[index] === "right"
                                        ? AlignmentType.RIGHT
                                        : AlignmentType.LEFT,
                                }),
                              ],
                              shading: rowIndex === 0 ? { fill: "F0EBE0" } : undefined,
                            }),
                        ),
                      ),
                    }),
                ),
              ),
            }),
          );
          break;
        case "code": {
          if (node.lang?.toLowerCase() === "mermaid") {
            const svg = await renderMermaid(node.value, false);
            const svgDocument = new DOMParser().parseFromString(svg, "image/svg+xml");
            const element = svgDocument.documentElement;
            const viewBox = element
              .getAttribute("viewBox")
              ?.trim()
              .split(/[\s,]+/)
              .map(Number);
            if (viewBox?.length === 4 && viewBox[2] > 0 && viewBox[3] > 0) {
              element.setAttribute("width", String(viewBox[2]));
              element.setAttribute("height", String(viewBox[3]));
            }
            // Chromium taints a canvas when a Blob SVG contains foreignObject
            // (Mermaid HTML labels). An inline SVG data URL stays origin-clean.
            const url = `data:image/svg+xml;charset=utf-8,${encodeURIComponent(
              new XMLSerializer().serializeToString(element),
            )}`;
            result.push(new Paragraph({ children: [await imageRun(url, "Mermaid")] }));
          } else {
            for (const line of node.value.split("\n"))
              result.push(
                new Paragraph({
                  children: [new TextRun({ text: line, font: "Consolas", size: 20 })],
                  shading: { fill: "F0EBE0" },
                  spacing: { after: 0 },
                  indent,
                }),
              );
          }
          break;
        }
        case "math":
        case "html":
          result.push(new Paragraph({ children: [new TextRun(node.value)], indent }));
          break;
        case "thematicBreak":
          result.push(new Paragraph({ thematicBreak: true }));
          break;
      }
    }
    return result;
  }

  const children = await blocks(tree.children);
  const document = new Document({
    title,
    styles: {
      default: {
        document: {
          run: { font: "Microsoft YaHei", size: 22, color: "000000" },
          paragraph: { spacing: { after: 120, line: 300 } },
        },
        heading1: { run: { color: "000000" } },
        heading2: { run: { color: "000000" } },
        heading3: { run: { color: "000000" } },
        heading4: { run: { color: "000000" } },
        heading5: { run: { color: "000000" } },
        heading6: { run: { color: "000000" } },
      },
    },
    numbering: { config: numbering },
    sections: [{ children: children.length ? children : [new Paragraph("")] }],
  });
  return new Uint8Array(await Packer.toArrayBuffer(document));
}
