export type NoteContextMenuAction = "pin" | "export" | "move" | "delete";

export interface NoteContextMenuItem {
  action: NoteContextMenuAction;
  label: string | ((pinned: boolean) => string);
  tone?: "danger";
}

export const noteContextMenuItems: NoteContextMenuItem[] = [
  { action: "pin", label: (pinned) => (pinned ? "取消置顶" : "置顶笔记") },
  { action: "export", label: "导出 Markdown" },
  { action: "move", label: "移动到分类…" },
  { action: "delete", label: "删除笔记", tone: "danger" },
];
