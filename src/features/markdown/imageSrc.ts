export type FileSrcConverter = (path: string) => string;

const NOTE_IMAGE_PREFIX = "images/";

// 远程地址（http/https/data 等带协议的链接）交给浏览器自行加载，不做本地路径处理
function isRemoteUrl(src: string): boolean {
  return /^[a-zA-Z][a-zA-Z0-9+.-]*:\/\//.test(src) || src.startsWith("data:");
}

// 判断是否为本地绝对路径：Windows 盘符（D:/...）或 UNC 路径（//server/share/...）
function isAbsoluteLocalPath(normalizedSrc: string): boolean {
  return /^[a-zA-Z]:\//.test(normalizedSrc) || normalizedSrc.startsWith("//");
}

// 笔记内部图片（粘贴/拖拽保存到 images/ 目录下的图片），走资源协议直接加载
export function resolveMarkdownImageSrc(
  src: string | undefined,
  imageBaseDir: string | undefined,
  convertFileSrc: FileSrcConverter,
): string {
  if (!src) {
    return "";
  }

  const normalizedSrc = src.replace(/\\/g, "/");
  if (!imageBaseDir || !normalizedSrc.startsWith(NOTE_IMAGE_PREFIX)) {
    return src;
  }

  return convertFileSrc(`${imageBaseDir}/${normalizedSrc}`);
}

// 外部 md 文档里引用的本地图片：不在应用自身的资源协议白名单目录下，
// 返回需要读取的绝对路径（供调用方走 data URL 通道异步加载），不适用则返回 undefined
export function resolveExternalLocalImagePath(
  src: string | undefined,
  externalFileDir: string | undefined,
): string | undefined {
  if (!src || isRemoteUrl(src)) {
    return undefined;
  }

  const normalizedSrc = src.replace(/\\/g, "/");

  if (isAbsoluteLocalPath(normalizedSrc)) {
    return normalizedSrc;
  }

  if (externalFileDir) {
    return `${externalFileDir.replace(/\\/g, "/")}/${normalizedSrc}`;
  }

  return undefined;
}
