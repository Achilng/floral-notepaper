import JSZip from "jszip";
import { describe, expect, test } from "vitest";
import { markdownToDocx } from "./docx";

async function unpack(content: string) {
  const bytes = await markdownToDocx(content, "中文笔记");
  expect(Array.from(bytes.slice(0, 2))).toEqual([80, 75]);
  const zip = await JSZip.loadAsync(bytes);
  return {
    xml: await zip.file("word/document.xml")!.async("string"),
    numbering: await zip.file("word/numbering.xml")!.async("string"),
    links: await zip.file("word/_rels/document.xml.rels")!.async("string"),
    metadata: await zip.file("docProps/core.xml")!.async("string"),
  };
}

describe("Markdown to DOCX", () => {
  test("creates real Word headings, formatting, escaped text, tables and links", async () => {
    const { xml, links, metadata } = await unpack(
      "# 中文标题\n\n**粗体** *斜体* ~~删除~~ `代码` A < B & C\n\n[网站](https://example.com)\n\n| 名称 | 值 |\n| --- | ---: |\n| 中文 | 12 |\n\n```ts\nconst a = 1;\n```",
    );
    expect(xml).toContain('w:val="Heading1"');
    expect(xml).toContain("中文标题");
    expect(xml).toContain("<w:b/>");
    expect(xml).toContain("<w:i/>");
    expect(xml).toContain("<w:strike/>");
    expect(xml).toContain("&lt; B &amp; C");
    expect(xml).toContain("<w:tbl>");
    expect(xml).toContain("const a = 1;");
    expect(xml).toContain("<w:hyperlink");
    expect(links).toContain("https://example.com");
    expect(metadata).toContain("中文笔记");
    expect(xml).not.toContain("**粗体**");
  });

  test("preserves nested and restarted lists, tasks and reference links", async () => {
    const { xml, numbering, links } = await unpack(
      "3. 第三项\n   - 子项\n4. 第四项\n\n正文\n\n1. 重新编号\n\n- [x] 完成\n- [ ] 待办\n\n[参考][ref]\n\n[ref]: https://example.com/ref",
    );
    expect(numbering).toContain('w:start w:val="3"');
    expect(numbering).toContain('w:start w:val="1"');
    expect(xml).toContain('w:ilvl w:val="1"');
    expect(xml).toContain("☑ ");
    expect(xml).toContain("☐ ");
    expect(xml.match(/>第三项</g)).toHaveLength(1);
    expect(links).toContain("https://example.com/ref");
  });

  test("supports empty notes and preserves unsupported source instead of losing content", async () => {
    expect((await unpack("")).xml).toContain("<w:body>");
    const { xml } = await unpack("$$\nx^2 + y^2\n$$\n\n<div>HTML 文本</div>");
    expect(xml).toContain("x^2 + y^2");
    expect(xml).toContain("HTML 文本");
  });
});
