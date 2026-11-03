import { invoke } from "@tauri-apps/api/core";
import { open, save } from "@tauri-apps/plugin-dialog";
import { beforeEach, describe, expect, test, vi } from "vitest";
import { exportDocxNote, exportMarkdownNote, importMarkdownNote } from "./api";
import { markdownToDocx } from "./docx";

vi.mock("./docx", () => ({ markdownToDocx: vi.fn() }));

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

vi.mock("@tauri-apps/plugin-dialog", () => ({
  open: vi.fn(),
  save: vi.fn(),
}));

const mockedInvoke = vi.mocked(invoke);
const mockedOpen = vi.mocked(open);
const mockedSave = vi.mocked(save);

describe("importExport api", () => {
  beforeEach(() => {
    mockedInvoke.mockReset();
    mockedOpen.mockReset();
    mockedSave.mockReset();
    vi.mocked(markdownToDocx).mockReset();
  });

  test("imports the selected markdown path through Rust", async () => {
    mockedOpen.mockResolvedValue("D:\\notes\\外部笔记.md");
    mockedInvoke.mockResolvedValue({
      id: "note-1",
      title: "外部笔记",
      fileName: "note-1.md",
      createdAt: "2026-04-28T00:00:00Z",
      updatedAt: "2026-04-28T00:00:00Z",
      wordCount: 4,
      content: "# 标题\n正文",
    });

    const note = await importMarkdownNote();

    expect(open).toHaveBeenCalledWith({
      multiple: false,
      directory: false,
      filters: [{ name: "Markdown", extensions: ["md"] }],
    });
    expect(invoke).toHaveBeenCalledWith("notes_import_markdown", {
      path: "D:\\notes\\外部笔记.md",
      category: "",
    });
    expect(note?.id).toBe("note-1");
  });

  test("returns null when the file picker is cancelled", async () => {
    mockedOpen.mockResolvedValue(null);

    await expect(importMarkdownNote()).resolves.toBeNull();
    expect(invoke).not.toHaveBeenCalled();
  });

  test("exports a note to the selected markdown path", async () => {
    mockedSave.mockResolvedValue("D:\\exports\\读书笔记.md");
    mockedInvoke.mockResolvedValue(undefined);

    await expect(exportMarkdownNote({ id: "note-1", title: "读书笔记" })).resolves.toBe(true);

    expect(save).toHaveBeenCalledWith({
      defaultPath: "读书笔记.md",
      filters: [{ name: "Markdown", extensions: ["md"] }],
    });
    expect(invoke).toHaveBeenCalledWith("notes_export_markdown", {
      id: "note-1",
      path: "D:\\exports\\读书笔记.md",
    });
  });

  test("uses a safe markdown file name for export", async () => {
    mockedSave.mockResolvedValue(null);

    await exportMarkdownNote({ id: "note-1", title: "A/B:Test" });
    await exportMarkdownNote({ id: "note-2", title: "" });
    await exportMarkdownNote({ id: "note-3", title: `${"x".repeat(79)}😀` });

    expect(save).toHaveBeenNthCalledWith(1, {
      defaultPath: "A_B_Test.md",
      filters: [{ name: "Markdown", extensions: ["md"] }],
    });
    expect(save).toHaveBeenNthCalledWith(2, {
      defaultPath: "无标题笔记.md",
      filters: [{ name: "Markdown", extensions: ["md"] }],
    });
    expect(save).toHaveBeenNthCalledWith(3, {
      defaultPath: `${"x".repeat(79)}😀.md`,
      filters: [{ name: "Markdown", extensions: ["md"] }],
    });
    expect(invoke).not.toHaveBeenCalled();
  });

  test("exports saved content as binary DOCX with a Unicode destination", async () => {
    const path = "D:\\exports\\读书笔记.docx";
    const bytes = new Uint8Array([80, 75, 3, 4]);
    mockedSave.mockResolvedValue(path);
    mockedInvoke.mockResolvedValueOnce({ title: "读书笔记", content: "# 最新内容" });
    vi.mocked(markdownToDocx).mockResolvedValue(bytes);
    await expect(exportDocxNote({ id: "note-1", title: "A/B" }, "D:/notes")).resolves.toBe(true);
    expect(save).toHaveBeenCalledWith({
      defaultPath: "A_B.docx",
      filters: [{ name: "Word Document", extensions: ["docx"] }],
    });
    expect(invoke).toHaveBeenNthCalledWith(1, "notes_get", { id: "note-1" });
    expect(markdownToDocx).toHaveBeenCalledWith("# 最新内容", "读书笔记", "D:/notes");
    expect(invoke).toHaveBeenNthCalledWith(2, "notes_export_docx", bytes, {
      headers: { "x-export-path": encodeURIComponent(path) },
    });
  });

  test("does not read or write a note when DOCX save is cancelled", async () => {
    mockedSave.mockResolvedValue(null);
    await expect(exportDocxNote({ id: "note-1", title: "" })).resolves.toBe(false);
    expect(invoke).not.toHaveBeenCalled();
    expect(markdownToDocx).not.toHaveBeenCalled();
  });

  test("does not write an incomplete document after conversion failure", async () => {
    mockedSave.mockResolvedValue("D:/note.docx");
    mockedInvoke.mockResolvedValueOnce({ title: "笔记", content: "![图](missing.png)" });
    vi.mocked(markdownToDocx).mockRejectedValue(new Error("Image unavailable"));
    await expect(exportDocxNote({ id: "note-1", title: "笔记" })).rejects.toThrow(
      "Image unavailable",
    );
    expect(invoke).toHaveBeenCalledTimes(1);
  });
});
