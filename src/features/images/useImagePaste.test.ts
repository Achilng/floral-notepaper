import { describe, expect, test, vi } from "vitest";
import { getImageFiles, imageExtensionFromFile } from "./useImagePaste";

function createFile(name: string, type = "") {
  return new File(["image-data"], name, { type });
}

function createDataTransfer(files: File[], itemFiles: File[] = files): DataTransfer {
  return {
    files,
    items: itemFiles.map((file) => ({
      kind: "file",
      type: file.type,
      getAsFile: vi.fn(() => file),
    })),
  } as unknown as DataTransfer;
}

describe("useImagePaste helpers", () => {
  test("detects image extension from mime type", () => {
    expect(imageExtensionFromFile({ name: "clipboard", type: "image/png" })).toBe("png");
    expect(imageExtensionFromFile({ name: "clipboard", type: "image/jpeg" })).toBe("jpg");
  });

  test("falls back to file extension when clipboard file type is missing", () => {
    expect(imageExtensionFromFile({ name: "screenshot.PNG", type: "" })).toBe("png");
    expect(imageExtensionFromFile({ name: "photo.jpeg", type: "" })).toBe("jpeg");
  });

  test("reads image files from dataTransfer.files when items do not expose files", () => {
    const image = createFile("screenshot.png");
    const text = createFile("note.txt", "text/plain");
    const dataTransfer = createDataTransfer([image, text], []);

    expect(getImageFiles(dataTransfer)).toEqual([image]);
  });
});
