// @vitest-environment jsdom
import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, test, vi } from "vitest";
import { MarkdownPreview } from "./MarkdownPreview";
import { renderMermaid } from "./mermaid";

vi.mock("./mermaid", () => ({ renderMermaid: vi.fn() }));
vi.mock("@tauri-apps/plugin-opener", () => ({ openUrl: vi.fn() }));
Object.assign(globalThis, { IS_REACT_ACT_ENVIRONMENT: true });

let container: HTMLDivElement;
let root: Root;
const render = vi.mocked(renderMermaid);

async function preview(source: string) {
  await act(async () => {
    root.render(<MarkdownPreview content={`\`\`\`mermaid\n${source}\n\`\`\``} />);
  });
}

beforeEach(() => {
  render.mockReset();
  document.documentElement.dataset.theme = "light";
  container = document.createElement("div");
  document.body.append(container);
  root = createRoot(container);
});

afterEach(async () => {
  await act(async () => root.unmount());
  container.remove();
  vi.unstubAllGlobals();
});

describe("Mermaid preview", () => {
  test("renders SVG, copies source, and updates for the app theme", async () => {
    render.mockResolvedValue('<svg data-diagram="ok"></svg>');
    const writeText = vi.fn().mockResolvedValue(undefined);
    vi.stubGlobal("navigator", { clipboard: { writeText } });
    await preview("graph TD; A --> B");
    expect(container.querySelector('[role="img"] svg')).not.toBeNull();
    expect(render).toHaveBeenLastCalledWith("graph TD; A --> B\n", false);
    await act(async () => container.querySelector("button")!.click());
    expect(writeText).toHaveBeenCalledWith("graph TD; A --> B\n");
    await act(async () => {
      document.documentElement.dataset.theme = "dark";
    });
    expect(render).toHaveBeenLastCalledWith("graph TD; A --> B\n", true);
  });

  test("shows source on failure and recovers after editing", async () => {
    render.mockRejectedValueOnce(new Error("invalid diagram"));
    await preview("invalid diagram");
    expect(container.querySelector('[role="status"]')?.textContent).toContain("渲染失败");
    expect(container.querySelector("pre")?.textContent).toContain("invalid diagram");
    render.mockResolvedValue('<svg data-diagram="recovered"></svg>');
    await preview("graph TD; A --> B");
    expect(container.querySelector("svg")?.getAttribute("data-diagram")).toBe("recovered");
    expect(container.querySelector('[role="status"]')).toBeNull();
  });

  test("ignores a stale render that finishes after an edit", async () => {
    let finishOld!: (svg: string) => void;
    render.mockReturnValueOnce(new Promise((resolve) => (finishOld = resolve)));
    await preview("graph TD; old --> diagram");
    render.mockResolvedValue('<svg data-diagram="new"></svg>');
    await preview("graph TD; new --> diagram");
    await act(async () => finishOld('<svg data-diagram="old"></svg>'));
    expect(container.querySelector("svg")?.getAttribute("data-diagram")).toBe("new");
  });
});
