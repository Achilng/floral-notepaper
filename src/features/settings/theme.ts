import type { AppConfig, ThemeOption } from "./types";

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
  root.classList.add("theme-transition");
  root.setAttribute("data-theme", resolveTheme(option));
  setTimeout(() => root.classList.remove("theme-transition"), 400);
}

export function applyFont(fontName: string): void {
  const root = document.documentElement;
  
  if (fontName && fontName.trim() !== "") {
    // 设置自定义字体
    root.style.setProperty('--font-body', `"${fontName}", system-ui, sans-serif`);
    root.style.setProperty('--font-display', `"${fontName}", Georgia, serif`);
  } else {
    // 使用默认字体
    root.style.setProperty('--font-body', '"Noto Sans SC", "Source Han Sans SC", system-ui, sans-serif');
    root.style.setProperty('--font-display', '"Noto Serif SC", "Source Han Serif SC", Georgia, serif');
  }
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

export function applyAppConfig(config: AppConfig): void {
  applyTheme(config.theme as ThemeOption);
  applyFont(config.appFont || "");
}