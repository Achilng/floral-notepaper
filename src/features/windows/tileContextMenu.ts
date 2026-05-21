import { t, type TFunction } from "i18next";
import type { NoteSurfaceAction } from "./surfaceActions";

export interface TileContextMenuItem {
  action: NoteSurfaceAction;
  label: string;
  tone?: "danger";
}

export function getTileContextMenuItems(translate: TFunction = t): TileContextMenuItem[] {
  return [
    {
      action: "copy",
      label: translate("contextMenu.tile.copy", { defaultValue: "复制" }),
    },
    {
      action: "save",
      label: translate("contextMenu.tile.save", { defaultValue: "保存" }),
    },
    {
      action: "switchToPad",
      label: translate("contextMenu.tile.switchToPad", { defaultValue: "转为小窗" }),
    },
    {
      action: "close",
      label: translate("contextMenu.tile.close", { defaultValue: "关闭" }),
      tone: "danger",
    },
  ];
}
