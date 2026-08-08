import { convertFileSrc } from "@tauri-apps/api/core";
import { useCallback, useLayoutEffect, useMemo, useRef } from "react";
import type { RefObject } from "react";
import { useTranslation } from "react-i18next";
import { resolveMarkdownImageSrc } from "../markdown/imageSrc";
import { useImagePaste } from "./useImagePaste";
import { parseNoteContentParts } from "./noteImageReferences";
import type { NoteContentPart } from "./noteImageReferences";

type TextPart = Extract<NoteContentPart, { type: "text" }>;

interface TextPartEditorProps {
  part: TextPart;
  partIndex: number;
  fontSize: number;
  noteId: string | null;
  primary: boolean;
  primaryTextareaRef: RefObject<HTMLTextAreaElement | null>;
  hasPreviousImage: boolean;
  hasNextImage: boolean;
  registerRef: (partIndex: number, textarea: HTMLTextAreaElement | null) => void;
  onChange: (partIndex: number, value: string) => void;
  onRemoveAdjacentImage: (partIndex: number, direction: "previous" | "next") => void;
  onEnsureNoteSaved: () => Promise<string | null>;
  onInserted: (relativePaths: string[], cursorOffset: number) => void;
  onError: (message: string) => void;
  onArrowUpFromStart: () => void;
  markDirty: () => void;
}

function TextPartEditor({
  part,
  partIndex,
  fontSize,
  noteId,
  primary,
  primaryTextareaRef,
  hasPreviousImage,
  hasNextImage,
  registerRef,
  onChange,
  onRemoveAdjacentImage,
  onEnsureNoteSaved,
  onInserted,
  onError,
  onArrowUpFromStart,
  markDirty,
}: TextPartEditorProps) {
  const { t } = useTranslation();
  const textareaRef = useRef<HTMLTextAreaElement | null>(null);
  const collapseEmptyPart = part.value.length === 0 && hasNextImage;

  const setTextareaRef = useCallback(
    (textarea: HTMLTextAreaElement | null) => {
      textareaRef.current = textarea;
      registerRef(partIndex, textarea);
      if (primary) primaryTextareaRef.current = textarea;
    },
    [partIndex, primary, primaryTextareaRef, registerRef],
  );

  useLayoutEffect(() => {
    const textarea = textareaRef.current;
    if (!textarea) return;
    textarea.style.height = "0px";
    textarea.style.height = collapseEmptyPart
      ? "0px"
      : `${Math.max(textarea.scrollHeight, Math.ceil(fontSize * 1.8))}px`;
  }, [collapseEmptyPart, fontSize, part.value]);

  const { handlePaste, handleDrop, handleDragOver } = useImagePaste({
    noteId,
    textareaRef,
    setContent: (value) => onChange(partIndex, value),
    markDirty,
    onEnsureNoteSaved,
    onError,
    onInserted: (relativePaths, cursorOffset) =>
      onInserted(relativePaths, part.start + cursorOffset),
    t,
  });

  return (
    <textarea
      ref={setTextareaRef}
      data-tab-indent="true"
      data-note-text-part={partIndex}
      value={part.value}
      rows={1}
      onChange={(event) => {
        onChange(partIndex, event.target.value);
        markDirty();
      }}
      onPaste={handlePaste}
      onDrop={handleDrop}
      onDragOver={handleDragOver}
      onKeyDown={(event) => {
        const textarea = event.currentTarget;
        if (
          event.key === "Backspace" &&
          hasPreviousImage &&
          textarea.selectionStart === 0 &&
          textarea.selectionEnd === 0
        ) {
          event.preventDefault();
          onRemoveAdjacentImage(partIndex, "previous");
          return;
        }
        if (
          event.key === "Delete" &&
          hasNextImage &&
          textarea.selectionStart === textarea.value.length &&
          textarea.selectionEnd === textarea.value.length
        ) {
          event.preventDefault();
          onRemoveAdjacentImage(partIndex, "next");
          return;
        }
        if (
          primary &&
          event.key === "ArrowUp" &&
          textarea.selectionStart === textarea.selectionEnd &&
          !textarea.value.slice(0, textarea.selectionStart).includes("\n")
        ) {
          event.preventDefault();
          onArrowUpFromStart();
        }
      }}
      className={`block w-full shrink-0 resize-none overflow-hidden leading-relaxed text-ink-soft font-body ${collapseEmptyPart ? "min-h-0 p-0" : "min-h-7 pb-1"}`}
      style={{ fontSize: `${fontSize}px`, tabSize: `var(--tab-indent-size, 2)` }}
    />
  );
}

interface NoteInlineImageEditorProps {
  content: string;
  imageBaseDir: string;
  fontSize: number;
  noteId: string | null;
  focusOffset: number | null;
  primaryTextareaRef: RefObject<HTMLTextAreaElement | null>;
  setContent: (value: string) => void;
  markDirty: () => void;
  onEnsureNoteSaved: () => Promise<string | null>;
  onImagesInserted: (relativePaths: string[], cursorOffset: number) => void;
  onError: (message: string) => void;
  onArrowUpFromStart: () => void;
  onFocusRestored: () => void;
}

export function NoteInlineImageEditor({
  content,
  imageBaseDir,
  fontSize,
  noteId,
  focusOffset,
  primaryTextareaRef,
  setContent,
  markDirty,
  onEnsureNoteSaved,
  onImagesInserted,
  onError,
  onArrowUpFromStart,
  onFocusRestored,
}: NoteInlineImageEditorProps) {
  const parts = useMemo(() => parseNoteContentParts(content), [content]);
  const textareaRefs = useRef(new Map<number, HTMLTextAreaElement>());

  const registerRef = useCallback((partIndex: number, textarea: HTMLTextAreaElement | null) => {
    if (textarea) textareaRefs.current.set(partIndex, textarea);
    else textareaRefs.current.delete(partIndex);
  }, []);

  useLayoutEffect(() => {
    if (focusOffset == null) return;

    let targetIndex = parts.findIndex(
      (part) => part.type === "text" && focusOffset >= part.start && focusOffset <= part.end,
    );
    if (targetIndex < 0) {
      targetIndex = parts.findIndex((part) => part.type === "text" && part.start >= focusOffset);
    }
    if (targetIndex < 0) {
      for (let index = parts.length - 1; index >= 0; index -= 1) {
        if (parts[index].type === "text") {
          targetIndex = index;
          break;
        }
      }
    }

    const targetPart = parts[targetIndex];
    const textarea = textareaRefs.current.get(targetIndex);
    if (targetPart?.type === "text" && textarea) {
      const cursor = Math.max(0, Math.min(targetPart.value.length, focusOffset - targetPart.start));
      textarea.focus();
      textarea.setSelectionRange(cursor, cursor);
    }
    onFocusRestored();
  }, [focusOffset, onFocusRestored, parts]);

  const serializeParts = useCallback(
    (replacementIndex?: number, replacementValue?: string, removedIndex?: number) =>
      parts
        .filter((_, index) => index !== removedIndex)
        .map((part, index) => {
          if (index === replacementIndex && part.type === "text") return replacementValue ?? "";
          return part.type === "text" ? part.value : part.raw;
        })
        .join(""),
    [parts],
  );

  const handleTextChange = useCallback(
    (partIndex: number, value: string) => setContent(serializeParts(partIndex, value)),
    [serializeParts, setContent],
  );

  const removeAdjacentImage = useCallback(
    (partIndex: number, direction: "previous" | "next") => {
      const imageIndex = direction === "previous" ? partIndex - 1 : partIndex + 1;
      if (parts[imageIndex]?.type !== "image") return;
      setContent(serializeParts(undefined, undefined, imageIndex));
      markDirty();
    },
    [markDirty, parts, serializeParts, setContent],
  );

  let firstTextPartIndex = parts.findIndex(
    (part, index) =>
      part.type === "text" && !(part.value.length === 0 && parts[index + 1]?.type === "image"),
  );
  if (firstTextPartIndex < 0) {
    firstTextPartIndex = parts.findIndex((part) => part.type === "text");
  }

  return (
    <div data-notepad-inline-image-editor="true" className="flex-1 min-h-0 overflow-y-auto pb-2">
      {parts.map((part, index) =>
        part.type === "text" ? (
          <TextPartEditor
            key={`text-${part.start}-${index}`}
            part={part}
            partIndex={index}
            fontSize={fontSize}
            noteId={noteId}
            primary={index === firstTextPartIndex}
            primaryTextareaRef={primaryTextareaRef}
            hasPreviousImage={parts[index - 1]?.type === "image"}
            hasNextImage={parts[index + 1]?.type === "image"}
            registerRef={registerRef}
            onChange={handleTextChange}
            onRemoveAdjacentImage={removeAdjacentImage}
            onEnsureNoteSaved={onEnsureNoteSaved}
            onInserted={onImagesInserted}
            onError={onError}
            onArrowUpFromStart={onArrowUpFromStart}
            markDirty={markDirty}
          />
        ) : (
          <div key={`image-${part.start}-${part.reference.src}`} className="my-2 w-full shrink-0">
            <img
              src={resolveMarkdownImageSrc(part.reference.src, imageBaseDir, convertFileSrc)}
              alt={part.reference.alt}
              decoding="async"
              className="mx-auto block max-h-[min(14rem,45vh)] max-w-full rounded bg-paper-warm/30 object-contain"
            />
          </div>
        ),
      )}
    </div>
  );
}
