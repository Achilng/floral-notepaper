export type ViewMode = "edit" | "split" | "preview";

export type ThemeOption = "light" | "dark" | "system";

export type TileColorMode = "system" | "custom";

export interface AppConfig {
  locale: string;
  notesDir: string;
  globalShortcut: string;
  closeToTray: boolean;
  autostart: boolean;
  defaultViewMode: string;
  noteAutoSave: boolean;
  noteSurfaceAutoSave: boolean;
  tileColor: string;
  tileColorMode: TileColorMode;
  theme: ThemeOption;
  fontSize: number;
  surfaceFontSize: number;
  externalFileAutoSave: boolean;
  rememberSurfaceSize: boolean;
  tileCtrlClose: boolean;
  tileRenderMarkdown: boolean;
  surfaceWidth?: number;
  surfaceHeight?: number;
  toggleVisibilityShortcut: string;
}
