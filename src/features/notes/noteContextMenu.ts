import { t, type TFunction } from "i18next";

export type NoteContextMenuAction = "export" | "exportPDF" | "move" | "delete";

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
      action: "exportPDF",
      label: translate("noteMenu.exportPDF", { defaultValue: "导出 PDF" }),
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
