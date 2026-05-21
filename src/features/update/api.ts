import { invoke } from "@tauri-apps/api/core";

export function downloadUpdateAsset(
  url: string,
  fileName: string,
): Promise<string> {
  return invoke("download_update_asset", { url, fileName });
}
