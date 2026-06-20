import GithubSlugger from "github-slugger";

export interface OutlineItem {
  /** Heading level, 1–6. */
  level: number;
  /** Display text with inline markdown stripped (matches the rendered heading). */
  text: string;
  /** Anchor id, identical to what `rehype-slug` produces in the preview. */
  slug: string;
  /** Zero-based source line index, used to scroll the editor by line. */
  line: number;
  /** Character offset of the heading's line start, used to place the caret. */
  from: number;
}

/**
 * Strip inline markdown from a heading's source so the text matches what the
 * preview renders (which is what `rehype-slug` slugifies via hast-util-to-string).
 */
function stripInline(raw: string): string {
  let s = raw;
  // Images ![alt](url) -> alt, links [text](url) -> text.
  s = s.replace(/!\[([^\]]*)\]\([^)]*\)/g, "$1");
  s = s.replace(/\[([^\]]*)\]\([^)]*\)/g, "$1");
  // Reference links [text][ref] / [text] -> text.
  s = s.replace(/\[([^\]]*)\]\[[^\]]*\]/g, "$1");
  // Inline code `code` -> code.
  s = s.replace(/`([^`]*)`/g, "$1");
  // Emphasis / strong / strikethrough markers.
  s = s.replace(/(\*\*|__|\*|_|~~)/g, "");
  return s.trim();
}

/**
 * Parse the markdown headings out of `content`, skipping any inside fenced code
 * blocks (matching `scrollSync.parseBlocks`). Anchor ids are generated with the
 * same `github-slugger` instance `rehype-slug` uses, so clicking an outline item
 * resolves to the matching preview anchor — including duplicate-heading suffixes.
 */
export function parseOutline(content: string): OutlineItem[] {
  const lines = content.split("\n");
  const slugger = new GithubSlugger();
  const items: OutlineItem[] = [];
  let fence: string | null = null;
  let offset = 0;

  for (let i = 0; i < lines.length; i++) {
    const line = lines[i];
    const from = offset;
    offset += line.length + 1; // advance past this line (and its "\n") on every path
    const trimmed = line.trim();

    // Toggle in/out of fenced code blocks (``` or ~~~).
    const fenceMatch = trimmed.match(/^(```|~~~)/);
    if (fenceMatch) {
      if (fence === null) fence = fenceMatch[1];
      else if (trimmed.startsWith(fence)) fence = null;
      continue;
    }
    if (fence !== null) continue;

    const heading = line.match(/^(#{1,6})\s+(.*)$/);
    if (!heading) continue;

    // Drop optional closing hashes (`## Title ##`).
    const body = heading[2].replace(/\s+#+\s*$/, "").trim();
    if (!body) continue;

    const text = stripInline(body);
    items.push({
      level: heading[1].length,
      text,
      slug: slugger.slug(text),
      line: i,
      from,
    });
  }

  return items;
}

/**
 * The heading whose section contains `line` (0-based): the last heading at or
 * above it. Returns null when `line` precedes the first heading.
 */
export function activeHeadingByLine(items: OutlineItem[], line: number): string | null {
  let slug: string | null = null;
  for (const item of items) {
    if (item.line <= line) slug = item.slug;
    else break;
  }
  return slug;
}
