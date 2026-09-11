import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, test, vi } from "vitest";
import type { AppConfig } from "../features/settings/types";
import { BackgroundLayer } from "./BackgroundLayer";

vi.mock("@tauri-apps/api/core", () => ({
  convertFileSrc: vi.fn((path: string) => `asset://${path}`),
}));

describe("BackgroundLayer", () => {
  test("renders a configured custom wallpaper", () => {
    const config = {
      backgroundImagePath: "C:/data/backgrounds/wallpaper.png",
      backgroundFit: "repeat",
      backgroundDim: 0.35,
      backgroundBlur: 4,
      backgroundScale: 1.2,
      backgroundPositionX: 30,
      backgroundPositionY: 70,
    } as AppConfig;

    const markup = renderToStaticMarkup(<BackgroundLayer config={config} />);

    expect(markup).toContain("asset://C:/data/backgrounds/wallpaper.png");
    expect(markup).toContain("background-repeat:repeat");
    expect(markup).toContain("background-position:30% 70%");
    expect(markup).toContain("filter:blur(4px)");
    expect(markup).toContain("transform:scale(1.2)");
    expect(markup).toContain("opacity:0.35");
  });

  test("renders nothing when no wallpaper is configured", () => {
    expect(renderToStaticMarkup(<BackgroundLayer config={null} />)).toBe("");
  });
});
