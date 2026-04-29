import { invoke } from "@tauri-apps/api/core";
import { open, save } from "@tauri-apps/plugin-dialog";
import type { Note } from "../notes/types";

const markdownFilters = [{ name: "Markdown", extensions: ["md"] }];

interface ExportableNote {
  id: string;
  title: string;
}

export async function importMarkdownNote(): Promise<Note | null> {
  const path = await open({
    multiple: false,
    directory: false,
    filters: markdownFilters,
  });

  if (typeof path !== "string") {
    return null;
  }

  return invoke("notes_import_markdown", { path });
}

export async function exportMarkdownNote(note: ExportableNote): Promise<boolean> {
  const path = await save({
    defaultPath: markdownFileName(note.title),
    filters: markdownFilters,
  });

  if (typeof path !== "string") {
    return false;
  }

  await invoke("notes_export_markdown", { id: note.id, path });
  return true;
}

function markdownFileName(title: string): string {
  const safeTitle = safeFileStem(title) || "无标题笔记";
  return `${safeTitle}.md`;
}

function safeFileStem(value: string): string {
  return value
    .trim()
    .replace(/[<>:"/\\|?*\u0000-\u001f]+/g, "_")
    .replace(/\s+/g, "_")
    .replace(/_+/g, "_")
    .replace(/^_+|_+$/g, "")
    .slice(0, 80);
}
