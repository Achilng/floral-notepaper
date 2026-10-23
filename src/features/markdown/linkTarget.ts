// 预览内链接的导航目标解析：
// - http(s) 外链与 #锚点 由预览组件自行处理；
// - 其余文件链接（相对/绝对路径、file:// URL）交由宿主（MainWindow）打开，
//   这里负责分类、解码与路径拼接，全部为纯函数便于单测。

export interface FileLinkTarget {
  // 已解码、分隔符统一为 /、不含 #锚点、file:// 前缀已剥离的路径
  path: string;
  // 已解码的锚点 id（不含 #），仅当链接带 #fragment 时存在
  anchor?: string;
}

export type MarkdownLinkTarget =
  | { kind: "url" }
  | { kind: "anchor"; id: string }
  | { kind: "unsupported-scheme" }
  | ({ kind: "file" } & FileLinkTarget);

const HTTP_URL_RE = /^https?:\/\//i;
const SCHEME_RE = /^[a-zA-Z][a-zA-Z0-9+.-]*:/;
const FILE_URL_PREFIX = "file://";
const TEXT_FILE_RE = /\.(md|markdown|txt)$/i;

// markdown 链接里常见 %20 这类百分号编码；文件名本身含 % 时解码会抛错，退回原文
function safeDecode(value: string): string {
  try {
    return decodeURIComponent(value);
  } catch {
    return value;
  }
}

function normalizeSeparators(path: string): string {
  return path.replace(/\\/g, "/");
}

// 剥离前导 ./（含连续多个），否则 ./周报.md 这类链接无法与笔记标题对齐
function stripLeadingDotSlash(path: string): string {
  let result = path;
  while (result.startsWith("./")) {
    result = result.slice(2);
  }
  return result;
}

// 拆出 #锚点 并对路径做解码与分隔符归一；无锚点时不带 anchor 键
function parsePathWithAnchor(href: string): FileLinkTarget {
  const hashIndex = href.indexOf("#");
  const path = stripLeadingDotSlash(
    normalizeSeparators(safeDecode(hashIndex >= 0 ? href.slice(0, hashIndex) : href)),
  );
  if (hashIndex < 0) return { path };
  const anchor = safeDecode(href.slice(hashIndex + 1));
  return anchor ? { path, anchor } : { path };
}

// file:///D:/a.md → D:/a.md；file:///home/a.md → /home/a.md；
// file://server/share/a.md → //server/share/a.md（UNC）
function parseFileUrl(href: string): FileLinkTarget {
  const target = parsePathWithAnchor(href.slice(FILE_URL_PREFIX.length));
  let path = target.path;
  if (/^\/[A-Za-z]:\//.test(path)) {
    path = path.slice(1);
  } else if (!/^[A-Za-z]:\//.test(path) && !path.startsWith("/")) {
    path = `//${path}`;
  }
  return target.anchor ? { path, anchor: target.anchor } : { path };
}

export function classifyMarkdownLink(href: string | undefined): MarkdownLinkTarget | null {
  if (!href) return null;
  if (HTTP_URL_RE.test(href)) return { kind: "url" };
  if (href.startsWith("#")) return { kind: "anchor", id: safeDecode(href.slice(1)) };
  if (SCHEME_RE.test(href)) {
    if (href.toLowerCase().startsWith(FILE_URL_PREFIX)) {
      return { kind: "file", ...parseFileUrl(href) };
    }
    return { kind: "unsupported-scheme" };
  }
  return { kind: "file", ...parsePathWithAnchor(href) };
}

export function isAbsolutePath(path: string): boolean {
  return /^[A-Za-z]:/.test(path) || path.startsWith("/");
}

export function isTextFilePath(path: string): boolean {
  return TEXT_FILE_RE.test(path);
}

export function stripTextExtension(path: string): string {
  return path.replace(TEXT_FILE_RE, "");
}

export function isSamePath(left: string, right: string): boolean {
  return normalizeSeparators(left) === normalizeSeparators(right);
}

// 取父目录用于相对链接解析；无分隔符时返回空串
export function parentDirectory(path: string): string {
  const normalized = normalizeSeparators(path);
  const slashIndex = normalized.lastIndexOf("/");
  return slashIndex >= 0 ? normalized.slice(0, slashIndex) : "";
}

// 以 baseDir 为基准词法解析相对路径（. / ..），保留盘符与 UNC 前缀的语义；
// .. 不会回退到盘符/UNC 主机之前
export function joinLinkPath(baseDir: string, relative: string): string {
  const normalizedBase = normalizeSeparators(baseDir).replace(/\/+$/, "");
  const isUnc = normalizedBase.startsWith("//");
  const body = isUnc ? normalizedBase.slice(2) : normalizedBase;
  const parts = body.split("/").filter((part) => part.length > 0);

  for (const segment of normalizeSeparators(relative).split("/")) {
    if (!segment || segment === ".") continue;
    if (segment === "..") {
      if (parts.length > 1) parts.pop();
      continue;
    }
    parts.push(segment);
  }

  return `${isUnc ? "//" : ""}${parts.join("/")}`;
}

interface TitledNote {
  title: string;
  category: string;
}

// 内部笔记的存储文件名带 uuid 前缀，无法按路径互链；按标题匹配目标笔记，
// 优先命中当前分类，其次全库（笔记列表按更新时间倒序，先到先得）
export function findNoteByLinkTitle<T extends TitledNote>(
  notes: T[],
  linkPath: string,
  preferCategory = "",
): T | null {
  const stripped = stripTextExtension(linkPath);
  const titles = stripped === linkPath ? [linkPath] : [linkPath, stripped];
  for (const title of titles) {
    const inCategory = notes.find(
      (note) => note.title === title && note.category === preferCategory,
    );
    if (inCategory) return inCategory;
    const anyCategory = notes.find((note) => note.title === title);
    if (anyCategory) return anyCategory;
  }
  return null;
}
