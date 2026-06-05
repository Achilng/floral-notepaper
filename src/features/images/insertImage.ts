const IMAGE_MIME_TO_EXT: Record<string, string> = {
  "image/png": "png",
  "image/jpeg": "jpg",
  "image/gif": "gif",
  "image/webp": "webp",
  "image/bmp": "bmp",
  "image/svg+xml": "svg",
};

const IMAGE_EXTENSIONS = new Set(["png", "jpg", "jpeg", "gif", "webp", "bmp", "svg"]);

export function imageExtensionFromFile(file: Pick<File, "name" | "type">): string | null {
  const mimeExt = IMAGE_MIME_TO_EXT[file.type];
  if (mimeExt) return mimeExt;

  const extension = file.name.split(".").pop()?.toLowerCase();
  return extension && IMAGE_EXTENSIONS.has(extension) ? extension : null;
}

export function markdownImageText(relativePath: string, alt = ""): string {
  return `![${alt}](${relativePath})`;
}

export function insertMarkdownAtSelection(
  value: string,
  selectionStart: number,
  selectionEnd: number,
  markdown: string,
): { value: string; cursor: number } {
  const before = value.slice(0, selectionStart);
  const after = value.slice(selectionEnd);
  const needsLeadingNewline = before.length > 0 && !before.endsWith("\n");
  const insertion = (needsLeadingNewline ? "\n" : "") + markdown + "\n";

  return {
    value: before + insertion + after,
    cursor: before.length + insertion.length,
  };
}
