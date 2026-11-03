// @vitest-environment jsdom
import { beforeEach, expect, test, vi } from "vitest";
import mermaid from "mermaid";
import { renderMermaid } from "./mermaid";

vi.mock("mermaid", () => ({
  default: { initialize: vi.fn(), render: vi.fn() },
}));

beforeEach(() => vi.clearAllMocks());

test("serializes configuration and rendering, uses unique IDs, and removes temporary nodes", async () => {
  const ids: string[] = [];
  const themes: unknown[] = [];
  vi.mocked(mermaid.render).mockImplementation(async (id, _source, container) => {
    ids.push(id);
    expect(container?.isConnected).toBe(true);
    const config = vi.mocked(mermaid.initialize).mock.lastCall![0];
    themes.push(config?.theme);
    expect(config).toMatchObject({
      startOnLoad: false,
      securityLevel: "strict",
      suppressErrorRendering: true,
      layout: "elk",
      elk: { lineHops: true },
      look: "classic",
    });
    expect(config?.secure).toContain("securityLevel");
    await Promise.resolve();
    // No other render may change global config while this one is pending.
    expect(vi.mocked(mermaid.initialize).mock.lastCall![0]).toBe(config);
    return { svg: "<svg></svg>", diagramType: "flowchart" };
  });
  const before = document.body.childElementCount;
  await Promise.all([
    renderMermaid("graph TD; A-->B", false),
    renderMermaid("graph TD; C-->D", true),
  ]);
  expect(new Set(ids).size).toBe(2);
  expect(themes).toEqual(["default", "dark"]);
  expect(document.body.childElementCount).toBe(before);
});

test("cleans up failed diagrams and lets the next diagram render", async () => {
  vi.mocked(mermaid.render)
    .mockRejectedValueOnce(new Error("syntax error"))
    .mockResolvedValueOnce({ svg: "<svg></svg>", diagramType: "flowchart" });
  const before = document.body.childElementCount;
  await expect(renderMermaid("invalid", false)).rejects.toThrow("syntax error");
  await expect(renderMermaid("graph TD; A-->B", false)).resolves.toBe("<svg></svg>");
  expect(document.body.childElementCount).toBe(before);
});
