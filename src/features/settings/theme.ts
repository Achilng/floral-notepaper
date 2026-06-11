import type { ThemeOption } from "./types";

function resolveTheme(option: ThemeOption): "light" | "dark" {
  if (option === "system") {
    return window.matchMedia("(prefers-color-scheme: dark)").matches ? "dark" : "light";
  }
  return option;
}

export function applyTheme(option: ThemeOption): void {
  const root = document.documentElement;
  const resolved = resolveTheme(option);
  // Cache to localStorage so the blocking script in index.html can set
  // data-theme before first paint, preventing a flash of wrong theme.
  localStorage.setItem("theme-option", option);
  localStorage.setItem("theme-resolved", resolved);
  if (root.getAttribute("data-theme") !== resolved) {
    root.classList.add("theme-transition");
    root.setAttribute("data-theme", resolved);
    setTimeout(() => root.classList.remove("theme-transition"), 400);
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
