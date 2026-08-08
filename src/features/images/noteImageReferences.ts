export interface NoteImageReference {
  alt: string;
  src: string;
}

export type NoteContentPart =
  | { type: "text"; value: string; start: number; end: number }
  | { type: "image"; raw: string; reference: NoteImageReference; start: number; end: number };

const FENCE_OPEN_PATTERN = /^ {0,3}(`{3,}|~{3,})/;
const FENCE_CLOSE_PATTERN = /^ {0,3}(`{3,}|~{3,})\s*$/;
const NOTE_IMAGE_PATTERN =
  /^ {0,3}!\[([^\]]*)\]\(\s*<?(images[\\/][^)\s>]+)>?(?:\s+(?:"[^"]*"|'[^']*'|\([^)]*\)))?\s*\)\s*$/;

function parseImageLine(line: string): NoteImageReference | null {
  const imageMatch = line.match(NOTE_IMAGE_PATTERN);
  if (!imageMatch) return null;

  const src = imageMatch[2];
  const pathSegments = src.replace(/\\/g, "/").split("/");
  if (pathSegments.some((segment) => segment === "." || segment === "..")) return null;

  return { alt: imageMatch[1], src };
}

export function parseNoteContentParts(content: string): NoteContentPart[] {
  const parts: NoteContentPart[] = [];
  let fenceMarker: string | null = null;
  let textBuffer = "";
  let position = 0;

  const pushText = () => {
    const start = position;
    position += textBuffer.length;
    parts.push({ type: "text", value: textBuffer, start, end: position });
    textBuffer = "";
  };

  const lines = content.match(/[^\n]*(?:\n|$)/g)?.filter(Boolean) ?? [];

  for (const rawLine of lines) {
    const lineWithPossibleCr = rawLine.endsWith("\n") ? rawLine.slice(0, -1) : rawLine;
    const line = lineWithPossibleCr.endsWith("\r")
      ? lineWithPossibleCr.slice(0, -1)
      : lineWithPossibleCr;

    if (!fenceMarker) {
      const openingFence = line.match(FENCE_OPEN_PATTERN);
      if (openingFence) {
        fenceMarker = openingFence[1];
        textBuffer += rawLine;
        continue;
      }
    } else {
      const closingFence = line.match(FENCE_CLOSE_PATTERN);
      if (
        closingFence &&
        closingFence[1][0] === fenceMarker[0] &&
        closingFence[1].length >= fenceMarker.length
      ) {
        fenceMarker = null;
      }
      textBuffer += rawLine;
      continue;
    }

    const reference = fenceMarker ? null : parseImageLine(line);
    if (!reference) {
      textBuffer += rawLine;
      continue;
    }

    let separator = "";
    if (textBuffer.endsWith("\r\n")) {
      separator = "\r\n";
      textBuffer = textBuffer.slice(0, -2);
    } else if (textBuffer.endsWith("\n")) {
      separator = "\n";
      textBuffer = textBuffer.slice(0, -1);
    }

    pushText();
    const raw = separator + rawLine;
    const start = position;
    position += raw.length;
    parts.push({ type: "image", raw, reference, start, end: position });
  }

  pushText();
  return parts;
}

export function extractNoteImageReferences(content: string): NoteImageReference[] {
  return parseNoteContentParts(content)
    .filter((part): part is Extract<NoteContentPart, { type: "image" }> => part.type === "image")
    .map((part) => part.reference);
}

export function visibleNoteText(content: string): string {
  return parseNoteContentParts(content)
    .filter((part): part is Extract<NoteContentPart, { type: "text" }> => part.type === "text")
    .map((part) => part.value)
    .join("");
}
