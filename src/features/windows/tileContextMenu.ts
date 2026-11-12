import { t, type TFunction } from "i18next";
import type { NoteSurfaceAction } from "./surfaceActions";
import { DEFAULT_WINDOW_LAYER, type WindowLayer } from "./windowLayer";

export interface TileContextMenuItem {
  action: NoteSurfaceAction;
  label: string;
  tone?: "danger";
  checked?: boolean;
}

export interface TileContextMenuSeparator {
  separator: true;
}

export type TileContextMenuEntry = TileContextMenuItem | TileContextMenuSeparator;

export function getTileContextMenuItems(
  translate: TFunction = t,
  currentLayer: WindowLayer = DEFAULT_WINDOW_LAYER,
  autostartEnabled: boolean = false,
): TileContextMenuEntry[] {
  const layerLabel = (layer: WindowLayer, label: string) =>
    currentLayer === layer ? `✓ ${label}` : label;
  const autostartLabel = autostartEnabled
    ? `✓ ${translate("contextMenu.tile.autostart", { defaultValue: "开机自启" })}`
    : translate("contextMenu.tile.autostart", { defaultValue: "开机自启" });

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
    { separator: true },
    {
      action: "layerTop",
      label: layerLabel("top", translate("contextMenu.tile.layerTop", { defaultValue: "置顶" })),
    },
    {
      action: "layerNormal",
      label: layerLabel(
        "normal",
        translate("contextMenu.tile.layerNormal", { defaultValue: "默认层级" }),
      ),
    },
    {
      action: "layerBottom",
      label: layerLabel(
        "bottom",
        translate("contextMenu.tile.layerBottom", { defaultValue: "沉底" }),
      ),
    },
    {
      action: "autostart",
      label: autostartLabel,
    },
    { separator: true },
    {
      action: "close",
      label: translate("contextMenu.tile.close", { defaultValue: "取消钉屏" }),
      tone: "danger",
    },
  ];
}
