import { describe, expect, test } from "vitest";
import {
  imageExtensionFromFile,
  insertMarkdownAtSelection,
  markdownImageText,
} from "./insertImage";

describe("insertImage helpers", () => {
  test("resolves image extension from mime type", () => {
    expect(imageExtensionFromFile({ name: "clipboard", type: "image/png" })).toBe("png");
    expect(imageExtensionFromFile({ name: "clipboard", type: "image/jpeg" })).toBe("jpg");
  });

  test("falls back to file extension when mime type is missing", () => {
    expect(imageExtensionFromFile({ name: "photo.PNG", type: "" })).toBe("png");
    expect(imageExtensionFromFile({ name: "photo.jpeg", type: "" })).toBe("jpeg");
  });

  test("ignores unsupported file types", () => {
    expect(imageExtensionFromFile({ name: "document.pdf", type: "application/pdf" })).toBeNull();
    expect(imageExtensionFromFile({ name: "README", type: "" })).toBeNull();
  });

  test("builds markdown image text", () => {
    expect(markdownImageText("images/note/photo.png")).toBe("![](images/note/photo.png)");
  });

  test("inserts markdown image text at selection with line breaks", () => {
    expect(insertMarkdownAtSelection("hello world", 6, 11, "![](images/photo.png)")).toEqual({
      value: "hello \n![](images/photo.png)\n",
      cursor: 29,
    });
  });
});
