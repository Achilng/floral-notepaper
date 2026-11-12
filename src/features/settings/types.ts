export type ViewMode = "edit" | "split" | "preview";

export type ThemeOption = "light" | "dark" | "system";

export type TileColorMode = "system" | "custom";
export type BackgroundFit = "cover" | "contain" | "repeat";

export interface AppConfig {
  locale: string;
  dataDir: string;
  globalShortcut: string;
  closeToTray: boolean;
  autostart: boolean;
  /** 开机自动恢复的磁贴笔记 id 列表（由磁贴右键菜单维护） */
  autostartTiles?: string[];
  defaultViewMode: string;
  noteAutoSave: boolean;
  noteSurfaceAutoSave: boolean;
  tileColor: string;
  tileColorMode: TileColorMode;
  theme: ThemeOption;
  fontSize: number;
  surfaceFontSize: number;
  tabIndentSize: number;
  externalFileAutoSave: boolean;
  rememberSurfaceSize: boolean;
  tileCtrlClose: boolean;
  tileDoubleClickToEdit: boolean;
  tileSaveReturnsToPin: boolean;
  tileRenderMarkdown: boolean;
  renderHtmlMarkdown: boolean;
  splitScrollSync: boolean;
  surfaceWidth?: number;
  surfaceHeight?: number;
  toggleVisibilityShortcut: string;
  openAtCursor: boolean;
  backgroundImagePath?: string;
  backgroundFit?: BackgroundFit;
  backgroundDim?: number;
  backgroundBlur?: number;
  backgroundScale?: number;
  backgroundPositionX?: number;
  backgroundPositionY?: number;
}
