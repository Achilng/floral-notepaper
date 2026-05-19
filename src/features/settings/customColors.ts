import chroma from "chroma-js";
import type { ColorMode } from "./types";

const DEFAULT_ACCENT = "#2d5a3d";
const DEFAULT_TEXT_LIGHT = "#1a1a18";
const DEFAULT_TEXT_DARK = "#e5e1da";

const FULL_HEX = /^#?([0-9a-fA-F]{6})$/;
const SHORT_HEX = /^#?([0-9a-fA-F]{3})$/;

export function normalizeHexColor(value: string | null | undefined, fallback: string): string {
  const trimmed = value?.trim() ?? "";
  const fullMatch = trimmed.match(FULL_HEX);
  if (fullMatch) return `#${fullMatch[1].toLowerCase()}`;
  const shortMatch = trimmed.match(SHORT_HEX);
  if (shortMatch) {
    return `#${shortMatch[1].split("").map((c) => c + c).join("").toLowerCase()}`;
  }
  return fallback;
}

function isDarkTheme(): boolean {
  if (typeof document === "undefined") return false;
  return document.documentElement.getAttribute("data-theme") === "dark";
}

export function deriveAccentPalette(accent: string) {
  const base = chroma(accent);
  const [h, s, l] = base.hsl();
  const safeS = isNaN(s) ? 0.35 : s;
  const safeL = isNaN(l) ? 0.35 : l;

  const light = chroma.hsl(h, Math.min(safeS * 1.1, 1), Math.min(safeL + 0.1, 0.95)).hex();
  const mist = chroma.hsl(h, Math.min(safeS * 0.4, 1), Math.min(0.92, safeL + 0.5)).hex();
  const glow = chroma.hsl(h, Math.min(safeS * 0.5, 1), Math.min(0.88, safeL + 0.4)).hex();

  const darkBase = chroma.hsl(h, Math.min(safeS * 0.85, 1), Math.max(0.45, safeL + 0.15)).hex();
  const darkLight = chroma.hsl(h, Math.min(safeS * 0.9, 1), Math.max(0.55, safeL + 0.25)).hex();
  const darkMist = chroma.hsl(h, Math.min(safeS * 0.5, 1), Math.max(0.1, safeL - 0.2)).hex();
  const darkGlow = chroma.hsl(h, Math.min(safeS * 0.55, 1), Math.max(0.14, safeL - 0.15)).hex();

  return {
    light: { bamboo: accent, bambooLight: light, bambooMist: mist, bambooGlow: glow },
    dark: { bamboo: darkBase, bambooLight: darkLight, bambooMist: darkMist, bambooGlow: darkGlow },
  };
}

export function deriveTextPalette(textColor: string) {
  const base = chroma(textColor);
  const [h, s, l] = base.hsl();
  const safeS = isNaN(s) ? 0.05 : s;
  const safeL = isNaN(l) ? 0.1 : l;

  const soft = chroma.hsl(h, Math.min(safeS * 0.8, 1), Math.min(safeL + 0.12, 0.95)).hex();
  const faint = chroma.hsl(h, Math.min(safeS * 0.5, 1), Math.min(safeL + 0.3, 0.95)).hex();
  const ghost = chroma.hsl(h, Math.min(safeS * 0.3, 1), Math.min(safeL + 0.45, 0.95)).hex();

  const darkBase = chroma.hsl(h, Math.min(safeS * 0.6, 1), Math.max(0.85, 1 - safeL)).hex();
  const darkSoft = chroma.hsl(h, Math.min(safeS * 0.5, 1), Math.max(0.7, 1 - safeL - 0.1)).hex();
  const darkFaint = chroma.hsl(h, Math.min(safeS * 0.35, 1), Math.max(0.55, 1 - safeL - 0.25)).hex();
  const darkGhost = chroma.hsl(h, Math.min(safeS * 0.25, 1), Math.max(0.42, 1 - safeL - 0.35)).hex();

  return {
    light: { ink: textColor, inkSoft: soft, inkFaint: faint, inkGhost: ghost },
    dark: { ink: darkBase, inkSoft: darkSoft, inkFaint: darkFaint, inkGhost: darkGhost },
  };
}

export function applyCustomAccentColor(mode: ColorMode, color: string): void {
  const root = document.documentElement;
  if (mode !== "custom") {
    root.style.removeProperty("--color-bamboo");
    root.style.removeProperty("--color-bamboo-light");
    root.style.removeProperty("--color-bamboo-mist");
    root.style.removeProperty("--color-bamboo-glow");
    return;
  }
  const normalized = normalizeHexColor(color, DEFAULT_ACCENT);
  const palette = deriveAccentPalette(normalized);
  const theme = isDarkTheme() ? palette.dark : palette.light;
  root.style.setProperty("--color-bamboo", theme.bamboo);
  root.style.setProperty("--color-bamboo-light", theme.bambooLight);
  root.style.setProperty("--color-bamboo-mist", theme.bambooMist);
  root.style.setProperty("--color-bamboo-glow", theme.bambooGlow);
}

export function applyCustomTextColor(mode: ColorMode, color: string): void {
  const root = document.documentElement;
  if (mode !== "custom") {
    root.style.removeProperty("--color-ink");
    root.style.removeProperty("--color-ink-soft");
    root.style.removeProperty("--color-ink-faint");
    root.style.removeProperty("--color-ink-ghost");
    return;
  }
  const fallback = isDarkTheme() ? DEFAULT_TEXT_DARK : DEFAULT_TEXT_LIGHT;
  const normalized = normalizeHexColor(color, fallback);
  const palette = deriveTextPalette(normalized);
  const theme = isDarkTheme() ? palette.dark : palette.light;
  root.style.setProperty("--color-ink", theme.ink);
  root.style.setProperty("--color-ink-soft", theme.inkSoft);
  root.style.setProperty("--color-ink-faint", theme.inkFaint);
  root.style.setProperty("--color-ink-ghost", theme.inkGhost);
}
