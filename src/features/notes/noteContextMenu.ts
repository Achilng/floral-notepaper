export type NoteContextMenuAction = "export" | "move" | "delete";

export interface NoteContextMenuItem {
  action: NoteContextMenuAction;
  label: string;
  tone?: "danger";
}

export const noteContextMenuItems: NoteContextMenuItem[] = [
  { action: "export", label: "main.contextMenu.export" },
  { action: "move", label: "main.contextMenu.moveToCategory" },
  { action: "delete", label: "main.contextMenu.deleteNote", tone: "danger" },
];
