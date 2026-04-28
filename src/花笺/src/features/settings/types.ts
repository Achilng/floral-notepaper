export type ViewMode = "edit" | "split" | "preview";

export interface AppConfig {
  notesDir: string;
  globalShortcut: string;
  closeToTray: boolean;
  autostart: boolean;
  defaultViewMode: string;
  noteSurfaceAutoSave: boolean;
}
