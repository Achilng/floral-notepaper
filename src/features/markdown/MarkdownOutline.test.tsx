// @vitest-environment jsdom
import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, expect, test, vi } from "vitest";
import { MarkdownPreview } from "./MarkdownPreview";

vi.mock("@tauri-apps/plugin-opener", () => ({ openUrl: vi.fn() }));
Object.assign(globalThis, { IS_REACT_ACT_ENVIRONMENT: true });

let container: HTMLDivElement;
let root: Root;

beforeEach(() => {
  container = document.createElement("div");
  document.body.append(container);
  root = createRoot(container);
});

afterEach(async () => {
  await act(async () => root.unmount());
  container.remove();
});

test("outline is opt-in and disappears when leaving pure preview", async () => {
  await act(async () => root.render(<MarkdownPreview content="# 标题" />));
  expect(container.querySelector("nav")).toBeNull();
  await act(async () => root.render(<MarkdownPreview content="# 标题" showOutline />));
  expect(container.querySelector("nav")?.textContent).toContain("标题");
  await act(async () => root.render(<MarkdownPreview content="# 标题" showOutline={false} />));
  expect(container.querySelector("nav")).toBeNull();
});

test("uses rendered headings, preserves depth and navigates to the exact repeated heading", async () => {
  await act(async () =>
    root.render(
      <MarkdownPreview
        content={"# 标题\n\n### **子标题**\n\n# 标题\n\n###### 末级\n\n```md\n# 代码\n```"}
        showOutline
      />,
    ),
  );
  const buttons = container.querySelectorAll<HTMLButtonElement>("nav button");
  expect(Array.from(buttons, (button) => button.textContent)).toEqual([
    "标题",
    "子标题",
    "标题",
    "末级",
  ]);
  expect(buttons[1].style.paddingInlineStart).toBe("32px");
  const headings = container.querySelectorAll("h1");
  const scroll = vi.fn();
  headings[1].scrollIntoView = scroll;
  await act(async () => buttons[2].click());
  expect(scroll).toHaveBeenCalledWith({ behavior: "smooth", block: "start" });
});

test("refreshes on note changes and respects the HTML rendering setting", async () => {
  const content = "<h2>HTML 标题</h2>\n\n普通正文";
  await act(async () => root.render(<MarkdownPreview content={content} showOutline />));
  expect(container.querySelector("nav")?.textContent).toBe("暂无标题");
  await act(async () => root.render(<MarkdownPreview content={content} renderHtml showOutline />));
  expect(container.querySelector("nav button")?.textContent).toBe("HTML 标题");
  await act(async () => root.render(<MarkdownPreview content="新笔记" renderHtml showOutline />));
  expect(container.querySelector("nav button")).toBeNull();
  expect(container.querySelector("nav")?.textContent).toBe("暂无标题");
});
