import { t, type TFunction } from "i18next";

export type NoteContextMenuAction = "export" | "exportDocx" | "move" | "delete";

export interface NoteContextMenuItem {
  action: NoteContextMenuAction;
  label: string;
  tone?: "danger";
}

export function getNoteContextMenuItems(translate: TFunction = t): NoteContextMenuItem[] {
  return [
    {
      action: "export",
      label: translate("noteMenu.export", { defaultValue: "导出 Markdown" }),
    },
    {
      action: "exportDocx",
      label: translate("noteMenu.exportDocx", { defaultValue: "导出 DOCX" }),
    },
    {
      action: "move",
      label: translate("noteMenu.moveToCategory", { defaultValue: "移动到分类…" }),
    },
    {
      action: "delete",
      label: translate("noteMenu.delete", { defaultValue: "删除笔记" }),
      tone: "danger",
    },
  ];
}
