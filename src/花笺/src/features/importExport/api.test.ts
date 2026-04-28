import { invoke } from "@tauri-apps/api/core";
import { open, save } from "@tauri-apps/plugin-dialog";
import { beforeEach, describe, expect, test, vi } from "vitest";
import { exportMarkdownNote, importMarkdownNote } from "./api";

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

    await expect(
      exportMarkdownNote({ id: "note-1", title: "读书笔记" }),
    ).resolves.toBe(true);

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

    expect(save).toHaveBeenNthCalledWith(1, {
      defaultPath: "A_B_Test.md",
      filters: [{ name: "Markdown", extensions: ["md"] }],
    });
    expect(save).toHaveBeenNthCalledWith(2, {
      defaultPath: "无标题笔记.md",
      filters: [{ name: "Markdown", extensions: ["md"] }],
    });
    expect(invoke).not.toHaveBeenCalled();
  });
});
