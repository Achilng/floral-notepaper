// @vitest-environment jsdom
import JSZip from "jszip";
import { afterEach, beforeEach, expect, test, vi } from "vitest";
import { markdownToDocx } from "./docx";
import { renderMermaid } from "../markdown/mermaid";
import { convertFileSrc } from "@tauri-apps/api/core";

vi.mock("../markdown/mermaid", () => ({ renderMermaid: vi.fn() }));
vi.mock("@tauri-apps/api/core", () => ({
  convertFileSrc: vi.fn((path: string) => `asset://${path}`),
}));

const png =
  "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mP8/x8AAwMCAO+jRZkAAAAASUVORK5CYII=";
let imageSources: string[];
let failImage: boolean;

beforeEach(() => {
  imageSources = [];
  failImage = false;
  vi.stubGlobal(
    "Image",
    class {
      naturalWidth = 1200;
      naturalHeight = 600;
      onload?: () => void;
      onerror?: () => void;
      set src(value: string) {
        imageSources.push(value);
        queueMicrotask(() => (failImage ? this.onerror?.() : this.onload?.()));
      }
    },
  );
  vi.spyOn(HTMLCanvasElement.prototype, "getContext").mockReturnValue({
    fillStyle: "",
    fillRect: vi.fn(),
    drawImage: vi.fn(),
  } as unknown as CanvasRenderingContext2D);
  vi.spyOn(HTMLCanvasElement.prototype, "toDataURL").mockReturnValue(
    `data:image/png;base64,${png}`,
  );
  vi.stubGlobal("URL", {
    createObjectURL: vi.fn(() => "blob:diagram"),
    revokeObjectURL: vi.fn(),
  });
  vi.mocked(renderMermaid).mockResolvedValue(
    '<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 1200 600" width="100%"><foreignObject width="1200" height="600"><div xmlns="http://www.w3.org/1999/xhtml">中文标签</div></foreignObject></svg>',
  );
});

afterEach(() => {
  vi.restoreAllMocks();
  vi.unstubAllGlobals();
  vi.clearAllMocks();
});

test("embeds local images and Mermaid as PNGs within the Word package", async () => {
  const bytes = await markdownToDocx(
    "![本地图](images/note/image.png)\n\n```mermaid\ngraph TD; A-->B\n```",
    "图片笔记",
    "D:/notes",
  );
  const zip = await JSZip.loadAsync(bytes);
  const xml = await zip.file("word/document.xml")!.async("string");
  expect(convertFileSrc).toHaveBeenCalledWith("D:/notes/images/note/image.png");
  expect(imageSources[0]).toBe("asset://D:/notes/images/note/image.png");
  expect(imageSources[1]).toMatch(/^data:image\/svg\+xml;charset=utf-8,/);
  const svg = decodeURIComponent(imageSources[1].split(",")[1]);
  expect(svg).toContain("foreignObject");
  expect(svg).toContain("中文标签");
  expect(svg).toContain('width="1200"');
  expect(svg).toContain('height="600"');
  expect(renderMermaid).toHaveBeenCalledWith("graph TD; A-->B", false);
  expect(xml.match(/<w:drawing>/g)).toHaveLength(2);
  expect(xml).toContain('cx="5715000"'); // 600px printable width
  expect(
    Object.keys(zip.files).some((path) => path.startsWith("word/media/") && path.endsWith(".png")),
  ).toBe(true);
  expect(URL.createObjectURL).not.toHaveBeenCalled();
});

test("reports missing media instead of silently dropping it", async () => {
  failImage = true;
  await expect(markdownToDocx("![丢失图片](missing.png)", "笔记")).rejects.toThrow("丢失图片");
});

test("reports Mermaid rendering failures", async () => {
  vi.mocked(renderMermaid).mockRejectedValueOnce(new Error("Invalid Mermaid"));
  await expect(markdownToDocx("```mermaid\ninvalid\n```", "笔记")).rejects.toThrow(
    "Invalid Mermaid",
  );
});
