import type { NoteSurfaceAction } from "./surfaceActions";

export interface TileContextMenuItem {
  action: NoteSurfaceAction;
  label: string;
  tone?: "danger";
}

export const tileContextMenuItems: TileContextMenuItem[] = [
  { action: "copy", label: "复制" },
  { action: "save", label: "保存" },
  { action: "switchToPad", label: "转为小窗" },
  { action: "close", label: "关闭", tone: "danger" },
];
