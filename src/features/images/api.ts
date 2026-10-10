import { invoke } from "@tauri-apps/api/core";

export function saveImage(noteId: string, data: Uint8Array, extension: string): Promise<string> {
  // 图片字节流作为 raw payload 直传（元数据走 headers），避免把二进制
  // 展开成 JS 数字数组再 JSON 序列化造成的巨大瞬时内存开销
  return invoke("images_save", data, {
    headers: { "x-note-id": noteId, "x-image-ext": extension },
  });
}

export function saveImageFromPath(noteId: string, filePath: string): Promise<string> {
  return invoke("images_save_from_path", { noteId, filePath });
}

export function getImagesBaseDir(): Promise<string> {
  return invoke("images_get_base_dir");
}

export function cleanUnusedImages(noteId: string, content: string): Promise<string[]> {
  return invoke("images_clean_unused", { noteId, content });
}
