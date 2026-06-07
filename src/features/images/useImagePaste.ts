import { useCallback, useRef } from "react";
import type { TFunction } from "i18next";
import { saveImage } from "./api";

const MAX_IMAGE_SIZE = 20 * 1024 * 1024; // 20 MB

const MIME_TO_EXT: Record<string, string> = {
  "image/png": "png",
  "image/jpeg": "jpg",
  "image/gif": "gif",
  "image/webp": "webp",
  "image/bmp": "bmp",
  "image/svg+xml": "svg",
};

const IMAGE_EXTENSIONS = new Set(["png", "jpg", "jpeg", "gif", "webp", "bmp", "svg"]);

interface UseImagePasteOptions {
  noteId: string | null;
  textareaRef: React.RefObject<HTMLTextAreaElement | null>;
  setContent: (content: string) => void;
  markDirty: () => void;
  onEnsureNoteSaved: () => Promise<string | null>;
  disabled?: boolean;
  onError?: (message: string) => void;
  t?: TFunction;
}

async function processImageFile(file: File, noteId: string, t?: TFunction): Promise<string | null> {
  if (file.size > MAX_IMAGE_SIZE) {
    throw new Error(
      t?.("errors.imageTooLarge", { defaultValue: "图片文件过大（上限 20 MB）" }) ??
        "图片文件过大（上限 20 MB）",
    );
  }

  const ext = imageExtensionFromFile(file);
  if (!ext) return null;

  const buffer = await file.arrayBuffer();
  const data = Array.from(new Uint8Array(buffer));
  return saveImage(noteId, data, ext);
}

export function imageExtensionFromFile(file: Pick<File, "name" | "type">): string | null {
  const mimeExt = MIME_TO_EXT[file.type];
  if (mimeExt) return mimeExt;

  const extension = file.name.split(".").pop()?.toLowerCase();
  return extension && IMAGE_EXTENSIONS.has(extension) ? extension : null;
}

function isImageFile(file: Pick<File, "name" | "type">): boolean {
  return imageExtensionFromFile(file) !== null;
}

function insertTextAtCursor(
  textarea: HTMLTextAreaElement,
  setContent: (value: string) => void,
  text: string,
) {
  const { selectionStart, selectionEnd, value } = textarea;
  const before = value.slice(0, selectionStart);
  const after = value.slice(selectionEnd);
  const needsLeadingNewline = before.length > 0 && !before.endsWith("\n");
  const insertion = (needsLeadingNewline ? "\n" : "") + text + "\n";
  const newContent = before + insertion + after;
  setContent(newContent);

  requestAnimationFrame(() => {
    const newPos = before.length + insertion.length;
    textarea.setSelectionRange(newPos, newPos);
    textarea.focus();
  });
}

export function getImageFiles(dataTransfer: DataTransfer): File[] {
  const files: File[] = [];
  const seen = new Set<File>();

  for (let i = 0; i < dataTransfer.items.length; i++) {
    const item = dataTransfer.items[i];
    if (item.kind !== "file") continue;

    const file = item.getAsFile();
    if (file && isImageFile(file)) {
      files.push(file);
      seen.add(file);
    }
  }

  for (let i = 0; i < dataTransfer.files.length; i++) {
    const file = dataTransfer.files[i];
    if (seen.has(file) || !isImageFile(file)) continue;
    files.push(file);
  }

  return files;
}

export function useImagePaste({
  noteId,
  textareaRef,
  setContent,
  markDirty,
  onEnsureNoteSaved,
  disabled,
  onError,
  t,
}: UseImagePasteOptions) {
  const processingRef = useRef(false);

  const processFiles = useCallback(
    async (files: File[]) => {
      if (processingRef.current || files.length === 0) return;
      processingRef.current = true;

      try {
        let resolvedId = noteId;
        if (!resolvedId) {
          resolvedId = await onEnsureNoteSaved();
          if (!resolvedId) return;
        }

        const textarea = textareaRef.current;
        if (!textarea) return;

        const markdownLines: string[] = [];
        for (const file of files) {
          const relativePath = await processImageFile(file, resolvedId, t);
          if (relativePath) {
            markdownLines.push(`![](${relativePath})`);
          }
        }

        if (markdownLines.length > 0) {
          insertTextAtCursor(textarea, setContent, markdownLines.join("\n"));
          markDirty();
        }
      } catch (error) {
        const message =
          error instanceof Error
            ? error.message
            : (t?.("errors.imagePasteFailed", { defaultValue: "图片粘贴失败" }) ?? "图片粘贴失败");
        onError?.(message);
      } finally {
        processingRef.current = false;
      }
    },
    [noteId, textareaRef, setContent, markDirty, onEnsureNoteSaved, onError, t],
  );

  const handlePaste = useCallback(
    (event: React.ClipboardEvent<HTMLTextAreaElement>) => {
      if (disabled) return;
      const files = getImageFiles(event.clipboardData);
      if (files.length === 0) return;
      event.preventDefault();
      void processFiles(files);
    },
    [disabled, processFiles],
  );

  const handleDrop = useCallback(
    (event: React.DragEvent<HTMLTextAreaElement>) => {
      if (disabled) return;
      const files = getImageFiles(event.dataTransfer);
      if (files.length === 0) return;
      event.preventDefault();
      void processFiles(files);
    },
    [disabled, processFiles],
  );

  const handleDragOver = useCallback(
    (event: React.DragEvent<HTMLTextAreaElement>) => {
      if (disabled) return;
      const hasImage =
        Array.from(event.dataTransfer.items).some((item) => {
          if (item.kind !== "file") return false;
          const file = item.getAsFile();
          return file ? isImageFile(file) : item.type in MIME_TO_EXT;
        }) || Array.from(event.dataTransfer.files).some(isImageFile);
      if (hasImage) {
        event.preventDefault();
        event.dataTransfer.dropEffect = "copy";
      }
    },
    [disabled],
  );

  return { handlePaste, handleDrop, handleDragOver };
}
