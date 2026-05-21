export type NoteContextMenuAction = "export" | "move" | "delete";

export interface NoteContextMenuItem {
  action: NoteContextMenuAction;
  label: string;
  tone?: "danger";
}

export const noteContextMenuItems: NoteContextMenuItem[] = [
  { action: "export", label: "导出 Markdown" },
  { action: "move", label: "移动到分类…" },
  { action: "delete", label: "删除笔记", tone: "danger" },
];
