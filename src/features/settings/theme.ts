import type { ThemeOption } from "./types";

const DEFAULT_UI_FONT_FAMILY =
  '"Noto Sans SC", "Source Han Sans SC", system-ui, sans-serif';

function resolveTheme(option: ThemeOption): "light" | "dark" {
  if (option === "system") {
    return window.matchMedia("(prefers-color-scheme: dark)").matches
      ? "dark"
      : "light";
  }
  return option;
}

export function applyTheme(option: ThemeOption): void {
  const root = document.documentElement;
  const resolved = resolveTheme(option);
  if (root.getAttribute("data-theme") !== resolved) {
    root.classList.add("theme-transition");
    root.setAttribute("data-theme", resolved);
    setTimeout(() => root.classList.remove("theme-transition"), 400);
  }
}

export function applyUiFontFamily(fontFamily?: string): void {
  const nextFontFamily = fontFamily?.trim() || DEFAULT_UI_FONT_FAMILY;
  document.documentElement.style.setProperty("--color-ui-font", nextFontFamily);
  document.documentElement.style.setProperty("--font-body", nextFontFamily);
  document.documentElement.style.setProperty("--font-display", nextFontFamily);
}

let systemListener: (() => void) | null = null;

export function watchSystemTheme(option: ThemeOption): () => void {
  if (systemListener) {
    systemListener();
    systemListener = null;
  }

  if (option !== "system") return () => {};

  const mql = window.matchMedia("(prefers-color-scheme: dark)");
  const handler = () => applyTheme("system");
  mql.addEventListener("change", handler);

  const cleanup = () => {
    mql.removeEventListener("change", handler);
    systemListener = null;
  };
  systemListener = cleanup;
  return cleanup;
}
