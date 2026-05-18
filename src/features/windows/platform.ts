/**
 * Synchronous platform detection utilities.
 *
 * Uses navigator.platform for instant (no-flash) checks;
 * falls back to the Tauri platform_os command for authoritative results.
 */

let _platformCache: string | null = null;

/** Synchronous check — safe to call during render. */
export function isWindowsSync(): boolean {
  return (
    typeof navigator !== "undefined" && /win/i.test(navigator.platform)
  );
}

/** Synchronous check — safe to call during render. */
export function isLinuxSync(): boolean {
  return (
    typeof navigator !== "undefined" && /linux/i.test(navigator.platform)
  );
}

/**
 * Authoritative platform string via Tauri command.
 * Prefer the sync helpers for render-time decisions; use this for logging or
 * fine-grained branching.
 */
export async function getPlatformOs(): Promise<string> {
  if (_platformCache) return _platformCache;
  const { invoke } = await import("@tauri-apps/api/core");
  _platformCache = await invoke<string>("platform_os");
  return _platformCache;
}
