import { describe, expect, test } from "vitest";
import {
  classifyMarkdownLink,
  findNoteByLinkTitle,
  isAbsolutePath,
  isSamePath,
  isTextFilePath,
  joinLinkPath,
  parentDirectory,
  stripTextExtension,
} from "./linkTarget";

describe("classifyMarkdownLink", () => {
  test("classifies http and https links as external urls", () => {
    expect(classifyMarkdownLink("https://example.com/a")).toEqual({ kind: "url" });
    expect(classifyMarkdownLink("http://example.com")).toEqual({ kind: "url" });
    expect(classifyMarkdownLink("HTTPS://example.com")).toEqual({ kind: "url" });
  });

  test("classifies fragment links as in-document anchors", () => {
    expect(classifyMarkdownLink("#section")).toEqual({ kind: "anchor", id: "section" });
    expect(classifyMarkdownLink("#%E6%A0%87%E9%A2%98")).toEqual({
      kind: "anchor",
      id: "标题",
    });
  });

  test("ignores other url schemes", () => {
    expect(classifyMarkdownLink("mailto:a@b.com")).toEqual({ kind: "unsupported-scheme" });
    expect(classifyMarkdownLink("ftp://example.com/a.md")).toEqual({
      kind: "unsupported-scheme",
    });
  });

  test("returns null for empty hrefs", () => {
    expect(classifyMarkdownLink(undefined)).toBeNull();
    expect(classifyMarkdownLink("")).toBeNull();
  });

  test("treats plain relative paths as file links", () => {
    expect(classifyMarkdownLink("./weekly.md")).toEqual({ kind: "file", path: "weekly.md" });
    expect(classifyMarkdownLink("notes/周报.md")).toEqual({
      kind: "file",
      path: "notes/周报.md",
    });
  });

  test("decodes percent escapes and keeps literal percent signs", () => {
    expect(classifyMarkdownLink("./my%20note.md")).toEqual({
      kind: "file",
      path: "my note.md",
    });
    expect(classifyMarkdownLink("./100%.md")).toEqual({ kind: "file", path: "100%.md" });
  });

  test("normalizes windows separators and splits fragments", () => {
    expect(classifyMarkdownLink(".\\docs\\a.md#标题")).toEqual({
      kind: "file",
      path: "docs/a.md",
      anchor: "标题",
    });
    expect(classifyMarkdownLink("..\\b.md#%E6%A6%82%E8%BF%B0")).toEqual({
      kind: "file",
      path: "../b.md",
      anchor: "概述",
    });
  });

  test("parses file urls for windows drives, unix paths and unc shares", () => {
    expect(classifyMarkdownLink("file:///D:/docs/a.md")).toEqual({
      kind: "file",
      path: "D:/docs/a.md",
    });
    expect(classifyMarkdownLink("file://D:/docs/a.md")).toEqual({
      kind: "file",
      path: "D:/docs/a.md",
    });
    expect(classifyMarkdownLink("file:///home/user/a.md#sec")).toEqual({
      kind: "file",
      path: "/home/user/a.md",
      anchor: "sec",
    });
    expect(classifyMarkdownLink("file://server/share/a.md")).toEqual({
      kind: "file",
      path: "//server/share/a.md",
    });
  });
});

describe("path helpers", () => {
  test("isAbsolutePath detects drives, roots and unc paths", () => {
    expect(isAbsolutePath("D:/docs/a.md")).toBe(true);
    expect(isAbsolutePath("/home/user/a.md")).toBe(true);
    expect(isAbsolutePath("//server/share/a.md")).toBe(true);
    expect(isAbsolutePath("./a.md")).toBe(false);
    expect(isAbsolutePath("../a.md")).toBe(false);
    expect(isAbsolutePath("docs/a.md")).toBe(false);
  });

  test("isTextFilePath matches markdown and text extensions only", () => {
    expect(isTextFilePath("a.md")).toBe(true);
    expect(isTextFilePath("a.MARKDOWN")).toBe(true);
    expect(isTextFilePath("a.txt")).toBe(true);
    expect(isTextFilePath("a.pdf")).toBe(false);
    expect(isTextFilePath("a.mdx")).toBe(false);
    expect(isTextFilePath("noext")).toBe(false);
  });

  test("stripTextExtension removes markdown and text extensions", () => {
    expect(stripTextExtension("周报.md")).toBe("周报");
    expect(stripTextExtension("周报.txt")).toBe("周报");
    expect(stripTextExtension("2024.report.md")).toBe("2024.report");
    expect(stripTextExtension("周报")).toBe("周报");
  });

  test("isSamePath ignores separator style only", () => {
    expect(isSamePath("D:\\docs\\a.md", "D:/docs/a.md")).toBe(true);
    expect(isSamePath("D:/docs/a.md", "D:/docs/b.md")).toBe(false);
  });

  test("parentDirectory keeps the directory part of a path", () => {
    expect(parentDirectory("D:/docs/a.md")).toBe("D:/docs");
    expect(parentDirectory("D:\\docs\\sub\\a.md")).toBe("D:/docs/sub");
    expect(parentDirectory("a.md")).toBe("");
  });
});

describe("joinLinkPath", () => {
  test("resolves relative segments against the base directory", () => {
    expect(joinLinkPath("D:/docs", "a.md")).toBe("D:/docs/a.md");
    expect(joinLinkPath("D:/docs", "./a.md")).toBe("D:/docs/a.md");
    expect(joinLinkPath("D:/docs/notes", "../a.md")).toBe("D:/docs/a.md");
    expect(joinLinkPath("D:/docs", "sub/./b.md")).toBe("D:/docs/sub/b.md");
  });

  test("accepts windows-style separators on both sides", () => {
    expect(joinLinkPath("D:\\docs\\notes", "..\\a.md")).toBe("D:/docs/a.md");
  });

  test("never pops past the drive root or unc host", () => {
    expect(joinLinkPath("D:/docs", "../../../../a.md")).toBe("D:/a.md");
    expect(joinLinkPath("//server/share", "../../a.md")).toBe("//server/a.md");
  });

  test("keeps the unc prefix when the base is a unc path", () => {
    expect(joinLinkPath("//server/share/notes", "a.md")).toBe("//server/share/notes/a.md");
    expect(joinLinkPath("\\\\server\\share\\notes", "..\\a.md")).toBe("//server/share/a.md");
  });
});

describe("findNoteByLinkTitle", () => {
  const notes = [
    { id: "1", title: "周报", category: "工作" },
    { id: "2", title: "周报", category: "生活" },
    { id: "3", title: "会议记录", category: "工作" },
  ];

  test("matches a note title including or excluding the text extension", () => {
    expect(findNoteByLinkTitle(notes, "周报")?.id).toBe("1");
    expect(findNoteByLinkTitle(notes, "周报.md")?.id).toBe("1");
    expect(findNoteByLinkTitle(notes, "会议记录.md")?.id).toBe("3");
  });

  test("prefers notes in the same category", () => {
    expect(findNoteByLinkTitle(notes, "周报", "生活")?.id).toBe("2");
    expect(findNoteByLinkTitle(notes, "周报", "工作")?.id).toBe("1");
  });

  test("returns null when no title matches", () => {
    expect(findNoteByLinkTitle(notes, "不存在的笔记.md")).toBeNull();
    expect(findNoteByLinkTitle([], "周报")).toBeNull();
  });
});
